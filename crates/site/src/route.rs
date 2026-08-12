#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    Index,
    About,
    Projects,
    Resume,
}

impl Route {
    pub const ALL: [Route; 4] = [Route::Index, Route::About, Route::Projects, Route::Resume];

    pub fn path(&self) -> &'static str {
        match self {
            Route::Index => "/",
            Route::About => "/about/",
            Route::Projects => "/projects/",
            Route::Resume => "/resume/",
        }
    }

    pub fn output_path(&self) -> &'static str {
        match self {
            Route::Index => "index.html",
            Route::About => "about/index.html",
            Route::Projects => "projects/index.html",
            Route::Resume => "resume/index.html",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Route::Index => "Index",
            Route::About => "About",
            Route::Projects => "Projects",
            Route::Resume => "Resume",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_route_has_a_rooted_path_and_html_output() {
        assert_eq!(Route::ALL.len(), 4, "Demos must not exist until bucket 4");
        for r in Route::ALL {
            assert!(r.path().starts_with('/'), "{:?} path must be rooted", r);
            assert!(r.output_path().ends_with(".html"), "{:?} bad output", r);
            assert!(!r.label().is_empty());
        }
    }

    #[test]
    fn index_is_served_from_the_root() {
        assert_eq!(Route::Index.path(), "/");
        assert_eq!(Route::Index.output_path(), "index.html");
        assert_eq!(Route::About.path(), "/about/");
        assert_eq!(Route::About.output_path(), "about/index.html");
    }
}
