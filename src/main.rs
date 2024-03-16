use std::env;
use std::fs::{read_dir, File};
use std::io::{BufRead as _, BufReader};
use std::path::Path;

fn process_line(line: &str) -> usize {
    if line.to_lowercase().contains("todo") {
        1
    } else {
        0
    }
 }

fn process_file(path: impl AsRef<Path>) -> anyhow::Result<usize> {
   let file = File::open(path)?;
   let mut count = 0;
   let reader = BufReader::new(file);
   for line in reader.lines().filter_map(Result::ok) {
        count += process_line(&line);
   }
   Ok(count)
}

fn process_dir(path: impl AsRef<Path>) -> anyhow::Result<usize> {
   let path = path.as_ref();
   let entries = read_dir(path)?;

   let mut count = 0;
   for entry in entries.filter_map(Result::ok) {
      let Ok(ft) = entry.file_type() else { continue };
      if ft.is_dir() {
            count += process_dir(path.join(entry.file_name()))?;
      } else if ft.is_file() {
            count += process_file(path.join(entry.file_name()))?;
      }
   }

   Ok(count)
}

fn main() -> anyhow::Result<()> {
   let workspace = env::var("GITHUB_WORKSPACE")?;
   let count = process_dir(workspace)?;
   println!("Number of TODOs: {count}");
   Ok(())
}

#[cfg(test)]
mod tests {
   use super::*;
   use std::fs::create_dir;
   use std::io::{BufWriter, Write as _};
   use tempfile::tempdir;

   #[test]
   fn process_line_with_todo() {
      assert_eq!(process_line("// TODO assert this"), 1);
   }

   #[test]
   fn process_line_with_no_todo() {
      assert_eq!(process_line("fn main() {}"), 0);
   }

   fn mock_file(path: impl AsRef<Path>, text: &str) -> anyhow::Result<()> {
      let file = File::create(path)?;
      let mut out = BufWriter::new(file);
      writeln!(&mut out, "{text}")?;
      Ok(())
   }

   #[test]
   fn process_file_with_todos() {
      let root = tempdir().expect("failed to create tempdir");
      mock_file(
            root.path().join("test_file.rs"),
            r#"fn main() {{
   // TODO implement body
   // TODO to do something useful
}}"#,
      )
      .expect("failed to create test file");
      assert_eq!(process_file(root.path().join("test_file.rs")).ok(), Some(2));
   }

   #[test]
   fn process_file_with_no_todos() {
      let root = tempdir().expect("failed to create tempdir");
      mock_file(
            root.path().join("test_file.rs"),
            r#"fn main() {{
   // this body is intentionally left empty
}}"#,
      )
      .expect("failed to create test file");
      assert_eq!(process_file(root.path().join("test_file.rs")).ok(), Some(0));
   }

   #[test]
   fn process_dir_with_todos() {
      let root = tempdir().expect("failed to create tempdir");

      mock_file(
            root.path().join("test1.rs"),
            r#"fn main() {{
   // TODO implement body
   // TODO to do something useful
}}"#,
      )
      .expect("failed to create test file");

      mock_file(
            root.path().join("test2.rs"),
            r#"fn main() {{
   // this body is intentionally left empty
}}"#,
      )
      .expect("failed to create test file");

      let dir1 = root.path().join("dir1");
      create_dir(dir1).expect("failed to create test subdirectory");

      let dir2 = root.path().join("dir2");
      create_dir(&dir2).expect("failed to create test subdirectory");

      mock_file(dir2.join("mod.rs"), "// TODO implement module")
            .expect("failed to create test file");

      assert_eq!(process_dir(root.path()).ok(), Some(3));
   }
}