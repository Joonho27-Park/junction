use crate::document::Document;
use crate::config::Config;
use crate::gui::windows::logview::LogStore;
use crate::gui::windows::sidebar::SidebarWindow;
use crate::gui::windows::dispatch_window::DispatchWindow;
use crate::import;
use crate::gui;

pub struct App {
    pub document :Document,
    pub config :Config,
    pub log :LogStore,
    pub windows: Windows,
    pub background_jobs :BackgroundJobs,
    //    - TODO set window name
    //    - TODO font / font size?
}

#[derive(Clone)]
/// Wrapper for thread pool.
pub struct BackgroundJobs(threadpool::ThreadPool);

impl BackgroundJobs {
    pub fn new() -> Self { BackgroundJobs(threadpool::ThreadPool::new(2)) }

    /// Run the given function as a background job.
    pub fn execute(&mut self, job: impl FnOnce() + Send + 'static) {
        self.0.execute(job)
    }
}

pub struct Windows {
    pub config: bool,
    pub debug: bool,
    pub log: bool,
    pub quit: bool,
    pub vehicles: bool,
    pub sidebar: SidebarWindow,
    pub dispatch_window: DispatchWindow,
    pub sidebar_split: Option<f32>,
    pub diagram_split :Option<f32>,
    pub import_window :import::ImportWindow,
    pub synthesis_window :Option<gui::windows::synthesis::SynthesisWindow>,
}

impl Windows {
    pub fn closed(bg :BackgroundJobs) -> Self {
        Windows {
            config :false,
            debug: false,
            log: false,
            quit: false,
            vehicles: false,
            sidebar: SidebarWindow::new(),
            dispatch_window: DispatchWindow::new(),
            sidebar_split: None,

            diagram_split: None,

            import_window: import::ImportWindow::new(bg),
            synthesis_window: None,
        }
    }
}

pub trait BackgroundUpdates {
    fn check(&mut self);
}

pub trait UpdateTime {
    fn advance(&mut self, dt :f64);
}


