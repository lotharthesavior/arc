use arc::domain::user::aggregate::UserAggregate;
use arc::domain::user::commands::UserCommand;
use arc::helpers::config::DEFAULT_USER_SNAPSHOT_INTERVAL_EVENTS;
use arc_core::command_bus::{CommandBus, CommandContext, SnapshotPolicy};
use arc_core::event_bus::InProcessEventBus;
use arc_core::event_store::InMemoryEventStore;

#[tokio::test]
async fn user_command_bus_snapshots_every_50_events() {
    let bus = CommandBus::<UserAggregate>::new(
        Box::new(InMemoryEventStore::new()),
        Box::new(InProcessEventBus::new()),
    )
    .with_snapshot_policy(SnapshotPolicy::EveryNEvents(
        DEFAULT_USER_SNAPSHOT_INTERVAL_EVENTS,
    ));

    let id = "user-snapshot-50";
    bus.dispatch(
        UserCommand::RegisterUser {
            id: id.to_string(),
            name: "Initial".to_string(),
            email: "initial@example.com".to_string(),
            password_hash: "hash".to_string(),
        },
        CommandContext::for_actor("test"),
    )
    .await
    .unwrap();

    for index in 2..=DEFAULT_USER_SNAPSHOT_INTERVAL_EVENTS {
        bus.dispatch(
            UserCommand::UpdateProfile {
                id: id.to_string(),
                name: format!("Name {index}"),
            },
            CommandContext::for_actor("test"),
        )
        .await
        .unwrap();
    }

    let snapshot = bus
        .event_store()
        .load_snapshot(id)
        .await
        .unwrap()
        .expect("snapshot at version 50");

    assert_eq!(snapshot.aggregate_type, "User");
    assert_eq!(snapshot.version, DEFAULT_USER_SNAPSHOT_INTERVAL_EVENTS);
    assert_eq!(
        snapshot.state["name"],
        format!("Name {DEFAULT_USER_SNAPSHOT_INTERVAL_EVENTS}")
    );
    assert_eq!(snapshot.state["exists"], true);
    assert_eq!(snapshot.state["deleted"], false);
}
