//! Application-owned UI host and capability contribution contracts.

use actix_session::Session;
use actix_web::{http::StatusCode, HttpRequest, HttpResponse};
use serde::Serialize;
use std::collections::HashSet;
use tera::{Context, Tera};
use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
pub struct TemplateName(pub &'static str);

#[derive(Clone, Copy)]
pub struct TemplateDef {
    pub name: TemplateName,
    pub source: &'static str,
}

#[derive(Clone, Copy, Default)]
pub struct TemplateBundle {
    pub templates: &'static [TemplateDef],
}

#[derive(Clone)]
pub struct UiHost {
    pub owner: &'static str,
    pub templates: TemplateBundle,
    pub admin_layout: TemplateName,
    pub public_layout: TemplateName,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", content = "roles", rename_all = "snake_case")]
pub enum Audience {
    Authenticated,
    AnyRole(&'static [&'static str]),
}

impl Audience {
    fn visible(&self, roles: &[String]) -> bool {
        match self {
            Self::Authenticated => true,
            Self::AnyRole(required) => required.iter().any(|role| roles.iter().any(|r| r == role)),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct AdminNavItem {
    pub id: &'static str,
    pub label: &'static str,
    pub href: &'static str,
    pub order: i16,
    pub audience: Audience,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionMethod {
    Get,
    PostWithCsrf,
}

#[derive(Clone, Debug, Serialize)]
pub struct AdminAction {
    pub id: &'static str,
    pub label: &'static str,
    pub href: &'static str,
    pub method: ActionMethod,
    pub audience: Audience,
}

#[derive(Clone, Default)]
pub struct UiContribution {
    pub owner: &'static str,
    pub templates: TemplateBundle,
    pub navigation: Vec<AdminNavItem>,
    pub actions: Vec<AdminAction>,
    #[doc(hidden)]
    pub duplicate_host: bool,
}

impl UiContribution {
    pub(crate) fn duplicate_host(owner: &'static str) -> Self {
        Self {
            owner,
            duplicate_host: true,
            ..Self::default()
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum UiError {
    #[error("browser UI contributions from {0} require a registered UI host")]
    MissingHost(String),
    #[error("multiple UI hosts registered (including `{0}`)")]
    MultipleHosts(&'static str),
    #[error("template `{0}` is registered more than once")]
    DuplicateTemplate(String),
    #[error("capability `{owner}` may not register reserved template `{name}`")]
    ReservedTemplate { owner: &'static str, name: String },
    #[error("capability template `{name}` must use namespace `capabilities/{owner}/`")]
    InvalidNamespace { owner: &'static str, name: String },
    #[error("missing canonical template `{0}`")]
    MissingCanonical(String),
    #[error("invalid template registry: {0}")]
    InvalidTemplate(String),
    #[error("duplicate navigation id `{0}`")]
    DuplicateNavigation(String),
    #[error("duplicate action id `{0}`")]
    DuplicateAction(String),
    #[error("page context may not set reserved key `{0}`")]
    ReservedContext(String),
}

#[derive(Debug)]
pub struct UiRegistry {
    tera: Tera,
    admin_layout: TemplateName,
    public_layout: TemplateName,
    navigation: Vec<AdminNavItem>,
    actions: Vec<AdminAction>,
}

impl UiRegistry {
    pub fn build(
        host: Option<&UiHost>,
        contributions: &[UiContribution],
    ) -> Result<Option<Self>, UiError> {
        if contributions.iter().any(|c| c.duplicate_host) {
            return Err(UiError::MultipleHosts(
                contributions
                    .iter()
                    .find(|c| c.duplicate_host)
                    .unwrap()
                    .owner,
            ));
        }
        let Some(host) = host else {
            if contributions.is_empty() {
                return Ok(None);
            }
            return Err(UiError::MissingHost(
                contributions
                    .iter()
                    .map(|c| c.owner)
                    .collect::<Vec<_>>()
                    .join(", "),
            ));
        };
        let mut defs = Vec::new();
        let mut names = HashSet::new();
        for def in host.templates.templates {
            if !names.insert(def.name.0) {
                return Err(UiError::DuplicateTemplate(def.name.0.into()));
            }
            defs.push((def.name.0, def.source));
        }
        for contribution in contributions {
            for def in contribution.templates.templates {
                let name = def.name.0;
                if name.starts_with("layouts/") || name.starts_with("components/") {
                    return Err(UiError::ReservedTemplate {
                        owner: contribution.owner,
                        name: name.into(),
                    });
                }
                let prefix = format!("capabilities/{}/", contribution.owner);
                if !name.starts_with(&prefix) {
                    return Err(UiError::InvalidNamespace {
                        owner: contribution.owner,
                        name: name.into(),
                    });
                }
                if !names.insert(name) {
                    return Err(UiError::DuplicateTemplate(name.into()));
                }
                defs.push((name, def.source));
            }
        }
        for canonical in [host.admin_layout.0, host.public_layout.0] {
            if !names.contains(canonical) {
                return Err(UiError::MissingCanonical(canonical.into()));
            }
        }
        let mut tera = Tera::default();
        tera.add_raw_templates(defs)
            .map_err(|e| UiError::InvalidTemplate(e.to_string()))?;
        let mut navigation = contributions
            .iter()
            .flat_map(|c| c.navigation.clone())
            .collect::<Vec<_>>();
        unique(
            navigation.iter().map(|v| v.id),
            UiError::DuplicateNavigation,
        )?;
        navigation.sort_by_key(|v| (v.order, v.id));
        let mut actions = contributions
            .iter()
            .flat_map(|c| c.actions.clone())
            .collect::<Vec<_>>();
        unique(actions.iter().map(|v| v.id), UiError::DuplicateAction)?;
        actions.sort_by_key(|v| v.id);
        Ok(Some(Self {
            tera,
            admin_layout: host.admin_layout,
            public_layout: host.public_layout,
            navigation,
            actions,
        }))
    }

    pub fn admin_layout(&self) -> TemplateName {
        self.admin_layout
    }
    pub fn public_layout(&self) -> TemplateName {
        self.public_layout
    }

    pub fn render(&self, page: UiPage, request: &HttpRequest, session: &Session) -> HttpResponse {
        const RESERVED: &[&str] = &[
            "app_name",
            "environment",
            "current_identity",
            "admin_navigation",
            "admin_actions",
            "csrf_token",
            "admin_layout",
            "public_layout",
        ];
        for key in RESERVED {
            if page.context.contains_key(key) {
                return HttpResponse::InternalServerError()
                    .body(UiError::ReservedContext((*key).into()).to_string());
            }
        }
        let mut context = page.context;
        let identity = session
            .get::<arc_auth_identity::SessionIdentity>("arc_auth_identity")
            .ok()
            .flatten();
        let roles = identity.as_ref().map(|i| i.roles.as_slice()).unwrap_or(&[]);
        let navigation = self
            .navigation
            .iter()
            .filter(|v| identity.is_some() && v.audience.visible(roles))
            .collect::<Vec<_>>();
        let actions = self
            .actions
            .iter()
            .filter(|v| identity.is_some() && v.audience.visible(roles))
            .collect::<Vec<_>>();
        context.insert(
            "app_name",
            &std::env::var("APP_NAME").unwrap_or_else(|_| env!("CARGO_PKG_NAME").into()),
        );
        context.insert(
            "environment",
            &std::env::var("APP_ENV").unwrap_or_else(|_| "development".into()),
        );
        context.insert("current_identity", &identity);
        context.insert("admin_navigation", &navigation);
        context.insert("admin_actions", &actions);
        context.insert("csrf_token", &crate::helpers::csrf::get_csrf_token(session));
        context.insert("admin_layout", &self.admin_layout.0);
        context.insert("public_layout", &self.public_layout.0);
        context.insert("request_path", request.path());
        match self.tera.render(page.template.0, &context) {
            Ok(body) => HttpResponse::build(page.status)
                .content_type("text/html; charset=utf-8")
                .body(body),
            Err(error) => {
                tracing::error!(template=page.template.0, %error, "UI render failed");
                HttpResponse::InternalServerError().body("The page could not be rendered.")
            }
        }
    }
}

fn unique<'a>(
    values: impl Iterator<Item = &'a str>,
    error: fn(String) -> UiError,
) -> Result<(), UiError> {
    let mut seen = HashSet::new();
    for value in values {
        if !seen.insert(value) {
            return Err(error(value.into()));
        }
    }
    Ok(())
}

pub struct UiPage {
    pub template: TemplateName,
    pub title: String,
    pub context: Context,
    pub status: StatusCode,
}
impl UiPage {
    pub fn new(template: TemplateName, title: impl Into<String>) -> Self {
        let title = title.into();
        let mut context = Context::new();
        context.insert("title", &title);
        Self {
            template,
            title,
            context,
            status: StatusCode::OK,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FormMethod {
    Get,
    Post,
}
#[derive(Clone, Debug, Serialize)]
pub struct FormSpec {
    pub id: &'static str,
    pub action: String,
    pub method: FormMethod,
    pub fields: Vec<FieldSpec>,
    pub submit_label: String,
    pub error: Option<String>,
}
#[derive(Clone, Debug, Serialize)]
pub struct FieldSpec {
    pub name: &'static str,
    pub label: String,
    pub kind: FieldKind,
    pub value: FieldValue,
    pub required: bool,
    pub autocomplete: Option<&'static str>,
    pub help: Option<String>,
    pub error: Option<String>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FieldKind {
    Text,
    Email,
    Password,
    Hidden,
    Select { options: Vec<OptionSpec> },
    MultiSelect { options: Vec<OptionSpec> },
    Checkbox,
}
#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum FieldValue {
    Empty,
    Text(String),
    Bool(bool),
    Many(Vec<String>),
}
#[derive(Clone, Debug, Serialize)]
pub struct OptionSpec {
    pub value: String,
    pub label: String,
}
impl FormSpec {
    pub fn post(id: &'static str, action: impl Into<String>) -> Self {
        Self {
            id,
            action: action.into(),
            method: FormMethod::Post,
            fields: vec![],
            submit_label: "Save".into(),
            error: None,
        }
    }
    pub fn field(mut self, field: FieldSpec) -> Self {
        self.fields.push(field);
        self
    }
    pub fn submit(mut self, label: impl Into<String>) -> Self {
        self.submit_label = label.into();
        self
    }
}
impl FieldSpec {
    fn new(name: &'static str, label: impl Into<String>, kind: FieldKind) -> Self {
        Self {
            name,
            label: label.into(),
            kind,
            value: FieldValue::Empty,
            required: false,
            autocomplete: None,
            help: None,
            error: None,
        }
    }
    pub fn text(name: &'static str, label: impl Into<String>) -> Self {
        Self::new(name, label, FieldKind::Text)
    }
    pub fn email(name: &'static str, label: impl Into<String>) -> Self {
        Self::new(name, label, FieldKind::Email)
    }
    pub fn password(name: &'static str, label: impl Into<String>) -> Self {
        Self::new(name, label, FieldKind::Password)
    }
    pub fn value(mut self, value: impl Into<String>) -> Self {
        if !matches!(self.kind, FieldKind::Password) {
            self.value = FieldValue::Text(value.into())
        }
        self
    }
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }
    pub fn autocomplete(mut self, v: &'static str) -> Self {
        self.autocomplete = Some(v);
        self
    }
}

// Avoid a dependency cycle: this wire shape deliberately matches arc-auth-core::Identity.
mod arc_auth_identity {
    use serde::{Deserialize, Serialize};
    #[derive(Deserialize, Serialize)]
    pub struct SessionIdentity {
        pub id: String,
        pub name: String,
        pub email: String,
        pub active: bool,
        pub roles: Vec<String>,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const HOST: &[TemplateDef] = &[
        TemplateDef {
            name: TemplateName("layouts/admin.html"),
            source: "{% block content %}{% endblock content %}",
        },
        TemplateDef {
            name: TemplateName("layouts/public.html"),
            source: "{% block content %}{% endblock content %}",
        },
    ];
    fn host() -> UiHost {
        UiHost {
            owner: "app",
            templates: TemplateBundle { templates: HOST },
            admin_layout: TemplateName("layouts/admin.html"),
            public_layout: TemplateName("layouts/public.html"),
        }
    }
    fn contribution(owner: &'static str) -> UiContribution {
        UiContribution {
            owner,
            templates: TemplateBundle::default(),
            navigation: vec![],
            actions: vec![],
            duplicate_host: false,
        }
    }

    #[test]
    fn api_only_needs_no_host() {
        assert!(UiRegistry::build(None, &[]).unwrap().is_none())
    }
    #[test]
    fn contribution_requires_host() {
        assert!(matches!(
            UiRegistry::build(None, &[contribution("auth")]),
            Err(UiError::MissingHost(_))
        ))
    }
    #[test]
    fn multiple_hosts_fail() {
        let duplicate = UiContribution::duplicate_host("other");
        assert_eq!(
            UiRegistry::build(Some(&host()), &[duplicate]).unwrap_err(),
            UiError::MultipleHosts("other")
        )
    }
    #[test]
    fn reserved_capability_template_fails() {
        const BAD: &[TemplateDef] = &[TemplateDef {
            name: TemplateName("layouts/bad.html"),
            source: "",
        }];
        let mut c = contribution("auth");
        c.templates = TemplateBundle { templates: BAD };
        assert!(matches!(
            UiRegistry::build(Some(&host()), &[c]),
            Err(UiError::ReservedTemplate { .. })
        ))
    }
    #[test]
    fn duplicate_navigation_fails() {
        let item = AdminNavItem {
            id: "same",
            label: "Same",
            href: "/",
            order: 0,
            audience: Audience::Authenticated,
        };
        let mut a = contribution("a");
        a.navigation.push(item.clone());
        let mut b = contribution("b");
        b.navigation.push(item);
        assert_eq!(
            UiRegistry::build(Some(&host()), &[a, b]).unwrap_err(),
            UiError::DuplicateNavigation("same".into())
        )
    }
    #[test]
    fn navigation_is_deterministic() {
        let mut c = contribution("a");
        c.navigation = vec![
            AdminNavItem {
                id: "z",
                label: "Z",
                href: "/z",
                order: 2,
                audience: Audience::Authenticated,
            },
            AdminNavItem {
                id: "a",
                label: "A",
                href: "/a",
                order: 1,
                audience: Audience::Authenticated,
            },
        ];
        let registry = UiRegistry::build(Some(&host()), &[c]).unwrap().unwrap();
        assert_eq!(
            registry.navigation.iter().map(|i| i.id).collect::<Vec<_>>(),
            vec!["a", "z"]
        )
    }
    #[test]
    fn password_values_are_suppressed() {
        let field = FieldSpec::password("password", "Password").value("secret");
        assert!(matches!(field.value, FieldValue::Empty))
    }
}
