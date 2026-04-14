use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_cli_config_get_llm_model() {
    let temp_dir = tempfile::Builder::new().prefix("awo-test-").tempdir_in(std::env::current_dir().unwrap()).unwrap();
    let temp_home = temp_dir.path().to_string_lossy().to_string();

    let mut cmd = Command::cargo_bin("agent-wiki-os").unwrap();
    cmd.env("HOME", &temp_home)
       .arg("config")
       .arg("get")
       .arg("llm.model")
       .assert()
       .success(); 
}

#[test]
fn test_cli_config_set_llm_model() {
    // 覆盖默认配置目录，避免在用户的真实 home 目录创建配置文件
    let temp_dir = tempfile::Builder::new().prefix("awo-test-").tempdir_in(std::env::current_dir().unwrap()).unwrap();
    let temp_home = temp_dir.path().to_string_lossy().to_string();

    let mut cmd = Command::cargo_bin("agent-wiki-os").unwrap();
    cmd.env("HOME", &temp_home) // Unix 系统的 HOME 环境变量
       .arg("config")
       .arg("set")
       .arg("llm.model")
       .arg("test-model-123")
       .assert()
       .success()
       .stdout(predicate::str::contains("Successfully set llm.model = 'test-model-123'"));

    let mut get_cmd = Command::cargo_bin("agent-wiki-os").unwrap();
    get_cmd.env("HOME", &temp_home)
           .arg("config")
           .arg("get")
           .arg("llm.model")
           .assert()
           .success()
           .stdout(predicate::str::contains("test-model-123"));
}
