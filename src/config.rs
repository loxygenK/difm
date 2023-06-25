use std::{path::PathBuf, fs::File, io::BufReader};

use serde::{Serialize, Deserialize};

pub fn read_config(path: Option<PathBuf>) -> ConfigContext {
    let path = path.unwrap_or("./difm.yaml".into());
    let file = File::open(&path).unwrap();
    let reader = BufReader::new(file);

    ConfigContext {
        config: serde_yaml::from_reader(reader).unwrap(),
        config_file: path.to_owned(),
    }
}

pub struct ConfigContext {
  pub config_file: PathBuf,
  pub config: Configuration,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Configuration {
  #[serde(alias = "task")]
  TaskDefinition(TaskDefinition)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskDefinition {
  #[serde(alias = "as")]
  pub alias: Option<String>,
  pub host: TaskHost,
  pub code: TaskCodeDefinition,
  pub run: Vec<TaskRun>,
  pub artifact: Vec<TaskArtifact>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskHost {
  pub name: String,
  pub base_dir: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskCodeDefinition {
  pub location: PathBuf,
  pub dest: PathBuf,
  pub ignore: String,

  #[serde(alias = "use")]
  pub protocol: TaskCodeProtocol,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskCodeProtocol {
  Ssh
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskRun {
  pub name: String,
  pub run: String,

  #[serde(default)]
  pub platform: TaskRunPlatform,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskRunPlatform {
  #[default]
  Remote
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskArtifact {
  pub remote_path: PathBuf,
  pub local_path: PathBuf
}