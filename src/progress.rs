use std::time::{Duration, Instant};

use spinners_rs::{Spinner, Spinners};

// ESEQ is for "escape sequence"
pub const ESEQ_DELETE_LINE: &str = "\x1b[0J";
pub const ESEQ_RED: &str = "\x1b[38;5;1m";
pub const ESEQ_GREEN: &str = "\x1b[38;5;2m";
pub const ESEQ_WEAK: &str = "\x1b[38;5;240m";
pub const ESEQ_RESET: &str = "\x1b[m";

pub const SPINNER_MS: u64 = 50;

pub struct ProgressView {
    task: String,
    spinner: Spinner,
    previous_update: Instant,
}

impl ProgressView {
    pub fn new(task: impl ToString) -> Self {
        let mut spinner = Spinner::new(Spinners::BouncingBar, task.to_string());
        spinner.set_interval(SPINNER_MS);

        Self {
            task: task.to_string(),
            spinner,
            previous_update: Instant::now(),
        }
    }

    pub fn with<T>(task: impl ToString, func: impl FnOnce(Self) -> T) -> T {
        let mut view = Self::new(task);
        view.start();

        func(view)
    }

    pub fn start(&mut self) {
        self.spinner.start();
    }

    pub fn update_task(&mut self, new_task: &str) {
        self.task = new_task.to_string();
        self.spinner.set_message(new_task);
    }

    pub fn report_intermediate(&mut self, progress: (usize, usize), comment: Option<&str>) {
        if self.previous_update.elapsed() <= Duration::from_millis(SPINNER_MS * 2) {
            return;
        }
        self.previous_update = Instant::now();

        self.spinner.set_message(format!(
            "{ESEQ_DELETE_LINE}[{}/{}] {}{}{ESEQ_RESET}",
            progress.0,
            progress.1,
            self.task,
            comment
                .map(|comment| format!("{ESEQ_WEAK} - {comment}"))
                .unwrap_or("".to_owned())
        ));
    }

    pub fn success(&mut self, message: Option<&str>) {
        self.previous_update = Instant::now();

        self.spinner.stop_with_message(format!(
            "{ESEQ_DELETE_LINE}{ESEQ_GREEN}✓ {}{}{ESEQ_RESET}",
            self.task,
            message
                .map(|message| format!(" - {}", message))
                .unwrap_or("".to_owned())
        ));
        println!();
    }

    pub fn failure(&mut self, message: Option<&str>) {
        self.previous_update = Instant::now();

        self.spinner.stop_with_message(format!(
            "{ESEQ_DELETE_LINE}{ESEQ_RED}! {}{}{ESEQ_RESET}",
            self.task,
            message
                .map(|message| format!(" - {}", message))
                .unwrap_or("".to_owned())
        ));
        println!();
    }

    pub fn stop(&mut self) {
        self.spinner.stop();
    }
}
