import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import YAML from "yaml";

import { writeBenthosConfig } from "./generate-benthos-config.mjs";

function writeManifest(repo, file, lines) {
  fs.writeFileSync(path.join(repo, "config", "handlers", file), lines.join("\n") + "\n");
}

function generatedPath(repo) {
  return path.join(repo, "config", "benthos", "generated", "events.yaml");
}

function withTempRepo(fn) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "arc-benthos-generator-"));
  fs.mkdirSync(path.join(dir, "config", "handlers"), { recursive: true });
  try {
    return fn(dir);
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
}

function runGenerator(repo) {
  writeBenthosConfig({ repoRoot: repo });
  return fs.readFileSync(path.join(repo, "config", "benthos", "generated", "events.yaml"), "utf8");
}

test("generates a safe stdout catch-all when no handler manifests exist", () => {
  withTempRepo((repo) => {
    const doc = YAML.parse(runGenerator(repo));
    assert.equal(doc.output.switch.cases.length, 2);
    assert.equal(doc.output.switch.cases[0].check, 'this.x_arc_dlq.handler == "envelope-validation"');
    assert.deepEqual(doc.output.switch.cases[1], { output: { stdout: {} } });
  });
});

test("generates envelope validation before dedupe and routes invalid envelopes to DLQ", () => {
  withTempRepo((repo) => {
    const doc = YAML.parse(runGenerator(repo));
    const processors = doc.pipeline.processors;

    assert.match(processors[1].mapping, /invalid Arc event envelope/);
    assert.deepEqual(processors[2].catch[0].mapping.includes('"handler": "envelope-validation"'), true);
    assert.equal(doc.output.switch.cases[0].output.nats_jetstream.subject, 'dlq.envelope.${! json("event_type").or("invalid").lowercase() }');
  });
});

test("generates an HTTP handler route with a DLQ fallback", () => {
  withTempRepo((repo) => {
    fs.writeFileSync(
      path.join(repo, "config", "handlers", "welcome-email.yaml"),
      [
        "name: welcome-email",
        "subscribe:",
        "  aggregate_types: [User]",
        "  event_types: [UserRegistered]",
        "delivery:",
        "  type: http",
        "  http:",
        '    url: "http://welcome-email:8090/handle"',
        "retry:",
        "  max_attempts: 2",
        "",
      ].join("\n"),
    );

    const doc = YAML.parse(runGenerator(repo));
    assert.equal(doc.output.switch.cases.length, 3);
    const handler = doc.output.switch.cases[1];

    assert.equal(
      handler.check,
      'this.x_arc_dlq == null && ["UserRegistered"].contains(this.event_type) && ["User"].contains(this.aggregate_type)',
    );
    const delivery = handler.output.fallback[0].retry;
    assert.equal(delivery.max_retries, 1);
    assert.equal(delivery.backoff.initial_interval, "2s");
    assert.equal(delivery.output.http_client.url, "http://welcome-email:8090/handle");
    assert.equal(
      handler.output.fallback[1].nats_jetstream.subject,
      'dlq.welcome-email.${! json("event_type").lowercase() }',
    );
    assert.match(handler.output.fallback[1].processors[0].mapping, /delivery_failed_after_retries/);
  });
});

test("generates a NATS handler route with a DLQ fallback", () => {
  withTempRepo((repo) => {
    writeManifest(repo, "audit-log.yaml", [
      "name: audit-log",
      "subscribe:",
      "  aggregate_types: [User]",
      "  event_types: [UserRegistered, UserDeleted]",
      "delivery:",
      "  type: nats",
      "  nats:",
      "    subject: handlers.audit-log",
    ]);

    const doc = YAML.parse(runGenerator(repo));
    assert.equal(doc.output.switch.cases.length, 3);
    const handler = doc.output.switch.cases[1];

    assert.equal(
      handler.check,
      'this.x_arc_dlq == null && ["UserRegistered", "UserDeleted"].contains(this.event_type) && ["User"].contains(this.aggregate_type)',
    );
    // dead_letter defaults to enabled, so the retried delivery output is wrapped in a fallback.
    const delivery = handler.output.fallback[0].retry.output;
    assert.equal(handler.output.fallback[0].retry.max_retries, 3);
    assert.equal(delivery.nats_jetstream.subject, "handlers.audit-log");
    assert.deepEqual(delivery.nats_jetstream.urls, ["${NATS_URL:nats://127.0.0.1:4222}"]);
    assert.equal(
      handler.output.fallback[1].nats_jetstream.subject,
      'dlq.audit-log.${! json("event_type").lowercase() }',
    );
  });
});

test("generates a SQL handler route with dsn env interpolation", () => {
  withTempRepo((repo) => {
    writeManifest(repo, "read-model.yaml", [
      "name: read-model",
      "subscribe:",
      "  aggregate_types: [User]",
      "  event_types: [UserRegistered]",
      "delivery:",
      "  type: sql",
      "  sql:",
      "    driver: postgres",
      "    dsn_env: READ_MODEL_DATABASE_URL",
      "    table: user_read_model",
      "    columns: [id, email, version]",
      '    args_mapping: "root = [this.aggregate_id, this.payload.email, this.sequence]"',
      '    suffix: "ON CONFLICT (id) DO UPDATE SET email = excluded.email, version = excluded.version"',
    ]);

    const doc = YAML.parse(runGenerator(repo));
    assert.equal(doc.output.switch.cases.length, 3);
    const insert = doc.output.switch.cases[1].output.fallback[0].retry.output.sql_insert;

    assert.equal(insert.driver, "postgres");
    assert.equal(insert.dsn, "${READ_MODEL_DATABASE_URL}");
    assert.equal(insert.table, "user_read_model");
    assert.deepEqual(insert.columns, ["id", "email", "version"]);
    assert.equal(
      insert.args_mapping,
      "root = [this.aggregate_id, this.payload.email, this.sequence]",
    );
    assert.equal(
      insert.suffix,
      "ON CONFLICT (id) DO UPDATE SET email = excluded.email, version = excluded.version",
    );
  });
});

test("check mode accepts an up-to-date generated config", () => {
  withTempRepo((repo) => {
    writeManifest(repo, "audit-log.yaml", [
      "name: audit-log",
      "subscribe:",
      "  event_types: [UserRegistered]",
      "delivery:",
      "  type: nats",
      "  nats:",
      "    subject: handlers.audit-log",
    ]);

    writeBenthosConfig({ repoRoot: repo });
    const result = writeBenthosConfig({ repoRoot: repo, check: true });
    assert.equal(result.changed, false);
  });
});

test("check mode rejects stale generated config", () => {
  withTempRepo((repo) => {
    writeManifest(repo, "audit-log.yaml", [
      "name: audit-log",
      "subscribe:",
      "  event_types: [UserRegistered]",
      "delivery:",
      "  type: nats",
      "  nats:",
      "    subject: handlers.audit-log",
    ]);

    writeBenthosConfig({ repoRoot: repo });
    // Diverge the committed artifact from what the manifests now generate.
    fs.appendFileSync(generatedPath(repo), "\n# hand-edited drift\n");

    assert.throws(() => writeBenthosConfig({ repoRoot: repo, check: true }), /out of date/);
  });
});

test("rejects manifests without subscribed event types", () => {
  withTempRepo((repo) => {
    fs.writeFileSync(
      path.join(repo, "config", "handlers", "bad.yaml"),
      ["name: bad", "subscribe:", "  aggregate_types: [User]", "delivery:", "  type: nats", "  nats:", "    subject: handlers.bad"].join(
        "\n",
      ),
    );

    assert.throws(
      () => writeBenthosConfig({ repoRoot: repo }),
      /subscribe\.event_types/,
    );
  });
});

test("rejects unsupported ordering guarantees instead of silently ignoring them", () => {
  withTempRepo((repo) => {
    writeManifest(repo, "ordered.yaml", [
      "name: ordered",
      "subscribe:",
      "  event_types: [UserRegistered]",
      "delivery:",
      "  type: nats",
      "  nats:",
      "    subject: handlers.ordered",
      "idempotency:",
      "  key: event_id",
      "  ordering: per_aggregate",
    ]);

    assert.throws(() => writeBenthosConfig({ repoRoot: repo }), /ordering currently supports only none/);
  });
});

test("rejects unknown manifest keys", () => {
  withTempRepo((repo) => {
    writeManifest(repo, "typo.yaml", [
      "name: typo",
      "subscribe:",
      "  event_types: [UserRegistered]",
      "delivery:",
      "  type: nats",
      "  nats:",
      "    subject: handlers.typo",
      "dead_leter:",
      "  enabled: true",
    ]);

    assert.throws(() => writeBenthosConfig({ repoRoot: repo }), /dead_leter is not supported/);
  });
});

test("rejects unsupported retry backoff modes", () => {
  withTempRepo((repo) => {
    writeManifest(repo, "linear.yaml", [
      "name: linear",
      "subscribe:",
      "  event_types: [UserRegistered]",
      "delivery:",
      "  type: http",
      "  http:",
      '    url: "http://handler.test"',
      "retry:",
      "  backoff: linear",
    ]);

    assert.throws(() => writeBenthosConfig({ repoRoot: repo }), /supports only exponential/);
  });
});
