use std::env;
use std::fs::{read_dir, File};
use std::io::{BufRead as _, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

const TODO: &str = "TODO";

fn write_summary(file: impl Write, count: usize) -> anyhow::Result<()> {
   let mut out = BufWriter::new(file);
   writeln!(&mut out, "Number of TODOs: {count}")?;
   Ok(())
}

fn write_output(file: impl Write, count: usize) -> anyhow::Result<()> {
   let mut out = BufWriter::new(file);
   writeln!(&mut out, "count={count}")?;
   Ok(())
}

fn process_line(path: impl AsRef<Path>, lineno: usize, line: &str) -> usize {
   if let Some(col) = line.find(TODO) {
      let (_, msg) = line.split_at(col + TODO.len());
      println!(
            "::notice file={},line={},col={},title={TODO}::{}",
            path.as_ref().to_string_lossy(),
            lineno + 1,
            col + 1,
            msg.trim(),
      );

      return 1;
   }

   0
}

fn process_file(base: impl AsRef<Path>, path: impl AsRef<Path>) -> anyhow::Result<usize> {
   let base = base.as_ref();
   let path = path.as_ref();
   let file = File::open(base.join(path))?;

   let mut count = 0;
   let reader = BufReader::new(file);
   for (lineno, line) in reader
      .lines()
      .enumerate()
      .filter_map(|(lineno, line)| line.ok().map(|line| (lineno, line)))
   {
      count += process_line(path, lineno, &line);
   }

   Ok(count)
}

fn process_dir(base: impl AsRef<Path>, path: impl AsRef<Path>) -> anyhow::Result<usize> {
   let base = base.as_ref();
   let path = path.as_ref();
   let entries = read_dir(base.join(path))?;

   let mut count = 0;
   for entry in entries.filter_map(Result::ok) {
      let Ok(ft) = entry.file_type() else { continue };
      if ft.is_dir() {
            count += process_dir(base, path.join(entry.file_name()))?;
      } else if ft.is_file() {
            count += process_file(base, path.join(entry.file_name()))?;
      }
   }

   Ok(count)
}

fn main() -> anyhow::Result<()> {
   let workspace = env::var("GITHUB_WORKSPACE")?;

   let mut dir = PathBuf::new();
   let mut summary = false;
   for arg in std::env::args().skip(1) {
      match arg.split_once('=') {
            Some(("--dir", val)) => dir = val.into(),
            Some(("--summary", val)) => summary = val == "true",
            _ => {
               println!("::debug title={TODO}::Invalid argument {arg}; ignoring...",);
            }
      }
   }

   let count = process_dir(workspace, dir)?;
   println!("::notice title=TODO::Count: {count}");

   if summary {
      let step_summary = env::var("GITHUB_STEP_SUMMARY")?;
      let file = File::options()
            .create(true)
            .append(true)
            .open(step_summary)?;

      write_summary(file, count)?;
   }

   let output = env::var("GITHUB_OUTPUT")?;
   let file = File::create(output)?;
   write_output(file, count)?;

   Ok(())
}

#[cfg(test)]
mod tests {
   use super::*;
   use std::fs::create_dir;
   use tempfile::tempdir;

   #[test]
   fn process_line_with_todo() {
      assert_eq!(process_line("test.rs", 0, "// TODO assert this"), 1);
   }

   #[test]
   fn process_line_with_no_todo() {
      assert_eq!(process_line("test.rs", 0, "fn main() {}"), 0);
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
      assert_eq!(process_file(root, "test_file.rs").ok(), Some(2));
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
      assert_eq!(process_file(root, "test_file.rs").ok(), Some(0));
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

      assert_eq!(process_dir(root, "").ok(), Some(3));
   }
}

// run program: "act -W .github/workflows/dev.yaml --input dir=resources/test-assets"