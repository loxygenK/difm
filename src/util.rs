use std::{thread::Thread, sync::Arc, io::stdin};

use spinners_rs::{Spinner, Spinners};
use tokio::sync::Mutex;

#[macro_export]
macro_rules! check {
  ($cond: expr, $prompt: literal $(, $( $params: expr $(,)? )* )? ) => {
    if !$cond {
      eprintln!(concat!("[!] ", $prompt) $(, $( $params, )* )?);
    }
  }
}

pub fn read_from_stdin(hidden: bool, prompt: &str) -> String {
  if hidden {
    rpassword::prompt_password(prompt).unwrap()
  } else {
    print!("{}", prompt);
    let mut read = String::new();
    stdin().read_line(&mut read).unwrap();

    read
  }
}

pub fn create_spinner(prompt: impl ToString) -> Spinner {
  let mut spinner = Spinner::new(Spinners::Dots, prompt.to_string());
  spinner.set_interval(30);

  spinner
}

pub fn with_spinner<T>(prompt: impl ToString, op: impl FnOnce(&mut Spinner) -> T) -> T {
  let mut spinner = create_spinner(prompt);
  spinner.start();

  let returning = op(&mut spinner);
  println!("");

  returning
}