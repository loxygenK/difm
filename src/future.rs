pub trait RulParallelExt: Iterator {
  fn run_parallel(max: usize) -> ();
}