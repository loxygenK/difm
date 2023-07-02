use std::io::stdin;

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

pub fn indent_str(string: &str, level: usize) -> String {
    string.lines()
        .map(|line| format!("{:indent$} |  {}", "", line, indent=level))
        .collect::<Vec<_>>()
        .join("\n")
}
