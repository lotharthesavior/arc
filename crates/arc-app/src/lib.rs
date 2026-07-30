pub mod domain;
pub mod http {
    pub mod controllers {
        pub mod internal_projection_controller;
    }
}

pub mod helpers {
    // Framework helpers are owned by arc-web and consumed by version.
    pub use arc_web::helpers::config;
    pub mod database;

    #[cfg(test)]
    pub mod test {
        pub struct InMemoryTestGuard;

        impl Drop for InMemoryTestGuard {
            fn drop(&mut self) {
                arc_web::helpers::database::reset_pool();
            }
        }
    }
}
