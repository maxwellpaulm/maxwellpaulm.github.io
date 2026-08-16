#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    Index,
    About,
    Projects,
    Resume,
    Ask,
    Demos,
}

impl Route {
    /// The routes the site actually publishes. `Route::Ask` is deliberately
    /// absent: the ask terminal is parked, not deleted. Its page, styles,
    /// crate, corpus emitter, and every test stay compiled and green — the
    /// site just doesn't publish it.
    ///
    /// To unpark, reverse four commented-out lines: this one; the
    /// `ask/index.json` write in `build.rs`; the `build_wasm_crate` call in
    /// `scripts/build-wasm.sh`; and the two `test -f` assertions in
    /// `.github/workflows/deploy.yml`. Then delete
    /// `build::tests::parked_ask_terminal_publishes_nothing`, which exists
    /// to make a half-unpark fail loudly.
    pub const ALL: [Route; 5] = [
        Route::Index,
        Route::About,
        Route::Projects,
        Route::Resume,
        // Route::Ask,
        Route::Demos,
    ];

    pub fn path(&self) -> &'static str {
        match self {
            Route::Index => "/",
            Route::About => "/about/",
            Route::Projects => "/projects/",
            Route::Resume => "/resume/",
            Route::Ask => "/ask/",
            Route::Demos => "/demos/",
        }
    }

    pub fn output_path(&self) -> &'static str {
        match self {
            Route::Index => "index.html",
            Route::About => "about/index.html",
            Route::Projects => "projects/index.html",
            Route::Resume => "resume/index.html",
            Route::Ask => "ask/index.html",
            Route::Demos => "demos/index.html",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Route::Index => "Index",
            Route::About => "About",
            Route::Projects => "Projects",
            Route::Resume => "Resume",
            Route::Ask => "Ask",
            Route::Demos => "Demos",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_route_has_a_rooted_path_and_html_output() {
        assert_eq!(Route::ALL.len(), 5);
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

    #[test]
    fn demos_is_routed() {
        assert_eq!(Route::Demos.path(), "/demos/");
        assert_eq!(Route::Demos.output_path(), "demos/index.html");
        assert_eq!(Route::Demos.label(), "Demos");
    }
}
