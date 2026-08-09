use super::aggregate::{{Type}}Aggregate;
use super::commands::{{Type}}Command;
use super::projector::{{CONSTANT}}_VIEW;
use actix_session::Session;
use actix_web::{web, HttpRequest, HttpResponse};
use arc_core::command_bus::{CommandBus, CommandContext};
use arc_core::read_model_store::ReadModelStore;
use arc_web::helpers::csrf;
use serde::{Deserialize, Serialize};
use tera::Context;
use arc_web::ui::{AdminNavItem, Audience, TemplateBundle, TemplateDef, TemplateName, UiContribution, UiPage};
use arc_web::UiRegistry;

const TEMPLATES:&[TemplateDef]=&[
 TemplateDef{name:TemplateName("capabilities/app-{{module}}/collection.html"),source:include_str!("../../../resources/views/resources/{{module}}/collection.html")},
 TemplateDef{name:TemplateName("capabilities/app-{{module}}/detail.html"),source:include_str!("../../../resources/views/resources/{{module}}/detail.html")},
 TemplateDef{name:TemplateName("capabilities/app-{{module}}/form.html"),source:include_str!("../../../resources/views/resources/{{module}}/form.html")},
];
pub fn contribution()->UiContribution{UiContribution{owner:"app-{{module}}",templates:TemplateBundle{templates:TEMPLATES},navigation:vec![AdminNavItem{id:"resource-{{module}}",label:"{{Type}}",href:"/admin/{{view}}",order:100,audience:Audience::Authenticated}],actions:vec![],duplicate_host:false}}

#[derive(Serialize)]
struct ResourceFormState {
    id: String,
    name: String,
    version: String,
}

fn render(registry:&UiRegistry, req:&HttpRequest, session:&Session, name: &'static str, context: Context, status: actix_web::http::StatusCode) -> HttpResponse {
    registry.render(UiPage{template:TemplateName(name),title:"{{Type}}".into(),context,status},req,session)
}

#[derive(Deserialize)]
struct ListQuery {
    filter: Option<String>,
    sort: Option<String>,
    page: Option<usize>,
}

async fn collection(
    req: HttpRequest,
    query: web::Query<ListQuery>,
    session: Session,
    store: web::Data<dyn ReadModelStore>,
    registry: web::Data<UiRegistry>,
) -> HttpResponse {
    match store.list({{CONSTANT}}_VIEW).await {
        Ok(mut rows) => {
            if let Some(filter) = query.filter.as_deref().filter(|value| !value.is_empty()) {
                let needle = filter.to_ascii_lowercase();
                rows.retain(|row| {
                    row.get("name")
                        .and_then(|v| v.as_str())
                        .is_some_and(|name| name.to_ascii_lowercase().contains(&needle))
                });
            }
            if query.sort.as_deref() == Some("name_desc") {
                rows.sort_by_key(|row| {
                    std::cmp::Reverse(
                        row.get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                    )
                });
            } else {
                rows.sort_by_key(|row| {
                    row.get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string()
                });
            }
            let page = query.page.unwrap_or(1).max(1);
            let per_page = 20;
            let total = rows.len();
            let rows = rows
                .into_iter()
                .skip((page - 1) * per_page)
                .take(per_page)
                .collect::<Vec<_>>();
            let mut context = Context::new();
            context.insert("rows", &rows);
            context.insert("filter", &query.filter);
            context.insert("page", &page);
            context.insert("has_next", &(page * per_page < total));
            render(
                &registry, &req, &session, "capabilities/app-{{module}}/collection.html",
                context,
                actix_web::http::StatusCode::OK,
            )
        }
        Err(_) => render(
            &registry, &req, &session, "capabilities/app-{{module}}/collection.html",
            Context::new(),
            actix_web::http::StatusCode::SERVICE_UNAVAILABLE,
        ),
    }
}

async fn detail(
    req: HttpRequest,
    id: web::Path<String>,
    session: Session,
    store: web::Data<dyn ReadModelStore>,
    registry: web::Data<UiRegistry>,
) -> HttpResponse {
    match store.get({{CONSTANT}}_VIEW, id.as_str()).await {
        Ok(Some(row)) => {
            let mut context = Context::new();
            context.insert("row", &row);
            render(
                &registry, &req, &session, "capabilities/app-{{module}}/detail.html",
                context,
                actix_web::http::StatusCode::OK,
            )
        }
        Ok(None) => HttpResponse::NotFound().finish(),
        Err(_) => HttpResponse::ServiceUnavailable().finish(),
    }
}

async fn new_form(req: HttpRequest, session: Session, registry:web::Data<UiRegistry>) -> HttpResponse {
    let mut context = Context::new();
    context.insert("mode", "create");
    context.insert(
        "form_state",
        &ResourceFormState {
            id: String::new(),
            name: String::new(),
            version: String::new(),
        },
    );
    render(
        &registry, &req, &session, "capabilities/app-{{module}}/form.html",
        context,
        actix_web::http::StatusCode::OK,
    )
}

#[derive(Deserialize, Serialize)]
struct ResourceForm {
    id: Option<String>,
    name: String,
    version: Option<i64>,
    csrf_token: String,
}

async fn create(
    req: HttpRequest,
    form: web::Form<ResourceForm>,
    session: Session,
    bus: web::Data<CommandBus<{{Type}}Aggregate>>,
    registry: web::Data<UiRegistry>,
) -> HttpResponse {
    if !csrf::validate_and_regenerate_csrf_token(&session, &form.csrf_token) {
        return HttpResponse::Forbidden().finish();
    }
    let id = form.id.clone().unwrap_or_default();
    if id.trim().is_empty() || form.name.trim().is_empty() {
        return invalid_form(&registry, &req, &session, &form, "ID and name are required.", "create");
    }
    match bus
        .dispatch(
            {{Type}}Command::Create {
                id: id.clone(),
                name: form.name.clone(),
            },
            CommandContext::for_actor("browser-session"),
        )
        .await
    {
        Ok(_) => HttpResponse::SeeOther()
            .insert_header(("Location", format!("/admin/{{view}}/{id}")))
            .finish(),
        Err(error) => invalid_form(&registry, &req, &session, &form, &error.to_string(), "create"),
    }
}

async fn edit_form(
    req: HttpRequest,
    id: web::Path<String>,
    session: Session,
    store: web::Data<dyn ReadModelStore>,
    registry: web::Data<UiRegistry>,
) -> HttpResponse {
    match store.get({{CONSTANT}}_VIEW, id.as_str()).await {
        Ok(Some(row)) => {
            let mut context = Context::new();
            context.insert("mode", "edit");
            context.insert("row", &row);
            context.insert(
                "form_state",
                &ResourceFormState {
                    id: row["id"].as_str().unwrap_or_default().to_owned(),
                    name: row["name"].as_str().unwrap_or_default().to_owned(),
                    version: row["version"]
                        .as_i64()
                        .map(|version| version.to_string())
                        .unwrap_or_default(),
                },
            );
            render(
                &registry, &req, &session, "capabilities/app-{{module}}/form.html",
                context,
                actix_web::http::StatusCode::OK,
            )
        }
        _ => HttpResponse::NotFound().finish(),
    }
}

async fn update(
    req: HttpRequest,
    id: web::Path<String>,
    form: web::Form<ResourceForm>,
    session: Session,
    store: web::Data<dyn ReadModelStore>,
    bus: web::Data<CommandBus<{{Type}}Aggregate>>,
    registry: web::Data<UiRegistry>,
) -> HttpResponse {
    if !csrf::validate_and_regenerate_csrf_token(&session, &form.csrf_token) {
        return HttpResponse::Forbidden().finish();
    }
    let current = store.get({{CONSTANT}}_VIEW, id.as_str()).await.ok().flatten();
    let current_version = current
        .as_ref()
        .and_then(|row| row.get("version"))
        .and_then(|v| v.as_i64());
    if current_version != form.version {
        return invalid_form(
            &registry, &req, &session,
            &form,
            "This record changed after you opened it. Reload and try again.",
            "edit",
        );
    }
    match bus
        .dispatch(
            {{Type}}Command::Rename {
                id: id.into_inner(),
                name: form.name.clone(),
            },
            CommandContext::for_actor("browser-session"),
        )
        .await
    {
        Ok(_) => HttpResponse::SeeOther()
            .insert_header((
                "Location",
                format!("/admin/{{view}}/{}", form.id.as_deref().unwrap_or("")),
            ))
            .finish(),
        Err(error) => invalid_form(&registry, &req, &session, &form, &error.to_string(), "edit"),
    }
}

fn invalid_form(registry:&UiRegistry, req:&HttpRequest, session: &Session, form: &ResourceForm, error: &str, mode: &str) -> HttpResponse {
    let mut context = Context::new();
    context.insert("mode", mode);
    context.insert("form", form);
    context.insert(
        "form_state",
        &ResourceFormState {
            id: form.id.clone().unwrap_or_default(),
            name: form.name.clone(),
            version: form
                .version
                .map(|version| version.to_string())
                .unwrap_or_default(),
        },
    );
    context.insert("error", error);
    render(
        registry, req, session, "capabilities/app-{{module}}/form.html",
        context,
        actix_web::http::StatusCode::UNPROCESSABLE_ENTITY,
    )
}

{{ui-auth-import}}{{ui-role-import}}
pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin/{{view}}")
            {{ui-role-wrap}}{{ui-auth-wrap}}
            .route("", web::get().to(collection))
            .route("/new", web::get().to(new_form))
            .route("/new", web::post().to(create))
            .route("/{id}", web::get().to(detail))
            .route("/{id}/edit", web::get().to(edit_form))
            .route("/{id}/edit", web::post().to(update)),
    );
}
