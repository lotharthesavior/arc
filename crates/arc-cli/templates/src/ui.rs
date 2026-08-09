use actix_session::Session;
use actix_web::{get, web, HttpRequest, HttpResponse, Responder};
use arc_web::ui::{AdminNavItem, Audience, TemplateBundle, TemplateDef, TemplateName, UiContribution, UiHost, UiPage};
use arc_web::UiRegistry;
use std::collections::HashMap;
use tera::Context;

const HOST_TEMPLATES: &[TemplateDef] = &[
    TemplateDef{name:TemplateName("home.html"),source:include_str!("../resources/views/home.html")},
    TemplateDef{name:TemplateName("layouts/public.html"),source:include_str!("../resources/views/layouts/public.html")},
    TemplateDef{name:TemplateName("layouts/admin.html"),source:include_str!("../resources/views/layouts/admin.html")},
    TemplateDef{name:TemplateName("components/ui.html"),source:include_str!("../resources/views/components/ui.html")},
    TemplateDef{name:TemplateName("admin/dashboard.html"),source:include_str!("../resources/views/admin/dashboard.html")},
];

pub fn host()->UiHost { UiHost{owner:env!("CARGO_PKG_NAME"),templates:TemplateBundle{templates:HOST_TEMPLATES},admin_layout:TemplateName("layouts/admin.html"),public_layout:TemplateName("layouts/public.html")} }
pub fn contribution()->UiContribution { UiContribution{owner:env!("CARGO_PKG_NAME"),templates:TemplateBundle::default(),navigation:vec![AdminNavItem{id:"app-overview",label:"Overview",href:"/admin",order:0,audience:Audience::Authenticated}],actions:vec![],duplicate_host:false} }
fn render(registry:&UiRegistry,req:&HttpRequest,session:&Session,name:&'static str,title:&str,mut context:Context)->HttpResponse{context.insert("title",title);registry.render(UiPage{template:TemplateName(name),title:title.into(),context,status:actix_web::http::StatusCode::OK},req,session)}

#[get("/")]
async fn home(req:HttpRequest,session:Session,registry:web::Data<UiRegistry>)->impl Responder { render(&registry,&req,&session,"home.html","Home",Context::new()) }
async fn dashboard(req:HttpRequest,session:Session,registry:web::Data<UiRegistry>)->impl Responder { let mut context=Context::new();context.insert("stats",&HashMap::from([("events",0),("projections",0)]));render(&registry,&req,&session,"admin/dashboard.html","Workbench",context) }
pub fn config(cfg:&mut web::ServiceConfig){cfg.service(home).service(web::scope("/admin")/* arc:admin-scope-middleware */.route("",web::get().to(dashboard)));}
