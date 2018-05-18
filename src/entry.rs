use itertools::Itertools;
use std::path::PathBuf;
use std::{fs, process};

use config::*;
use error::*;

/// An entry to be built.
#[derive(Debug, new)]
pub struct Entry {
    name: String,
    llvm: LLVM,
    clang: Clang,
    option: Option<CMakeOption>,
}

#[derive(Deserialize, Debug)]
pub struct CMakeOption {
    target: Option<Vec<String>>,
    example: Option<bool>,
    document: Option<bool>,
}

pub type URL = String;
pub type Branch = String;

#[derive(Debug)]
pub enum LLVM {
    SVN(URL, Branch),
    Git(URL, Branch),
}

#[derive(Debug)]
pub enum Clang {
    SVN(URL, Branch),
    Git(URL, Branch),
    None,
}

impl Entry {
    fn src_dir(&self) -> PathBuf {
        cache_dir().join(&self.name)
    }

    fn build_dir(&self) -> PathBuf {
        self.src_dir().join("build")
    }

    pub fn prefix(&self) -> PathBuf {
        data_dir().join(&self.name)
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn checkout(&self) -> Result<()> {
        if !cache_dir().exists() {
            fs::create_dir_all(cache_dir())?;
        }
        let src = self.src_dir();
        if !src.exists() {
            // clone/checkout
            match self.llvm {
                LLVM::SVN(ref url, ref _branch) => {
                    process::Command::new("svn")  // TODO support branch in SVN
                    .args(&["co", url.as_str()])
                    .arg(&self.name)
                    .current_dir(cache_dir())
                    .check_run()?
                }
                LLVM::Git(ref url, ref branch) => {
                    process::Command::new("git")
                        .args(&["clone", url.as_str()])
                        .arg(&self.name)
                        .current_dir(cache_dir())
                        .check_run()?;
                    process::Command::new("git")
                        .args(&["checkout", branch])
                        .current_dir(&src)
                        .check_run()?;
                }
            }
        }
        let tools = src.join("tools");
        let clang = tools.join("clang");
        if !clang.exists() {
            match self.clang {
                Clang::SVN(ref url, ref _branch) => {
                    process::Command::new("svn") // TODO support branch in SVN
                    .args(&["co", url.as_str(), "clang"])
                    .current_dir(tools)
                    .check_run()?
                }
                Clang::Git(ref url, ref branch) => {
                    process::Command::new("git")
                        .args(&["clone", url.as_str(), "clang"])
                        .current_dir(tools)
                        .check_run()?;
                    process::Command::new("git")
                        .args(&["checkout", branch])
                        .current_dir(&clang)
                        .check_run()?;
                }
                Clang::None => info!("No clang."),
            }
        }
        Ok(())
    }

    pub fn fetch(&self) -> Result<()> {
        let src = self.src_dir();
        if !src.exists() {
            match self.llvm {
                LLVM::SVN(_, _) => process::Command::new("svn")
                    .arg("update")
                    .current_dir(self.src_dir())
                    .check_run()?,
                LLVM::Git(_, _) => process::Command::new("git")
                    .arg("pull")
                    .current_dir(self.src_dir())
                    .check_run()?,
            };
        }
        let tools = src.join("tools");
        let clang = tools.join("clang");
        if !clang.exists() {
            match self.clang {
                Clang::SVN(_, _) => process::Command::new("svn")
                    .arg("update")
                    .current_dir(clang)
                    .check_run()?,
                Clang::Git(_, _) => process::Command::new("git")
                    .arg("pull")
                    .current_dir(clang)
                    .check_run()?,
                Clang::None => {}
            };
        }
        Ok(())
    }

    pub fn build(&self, nproc: usize) -> Result<()> {
        let build = self.build_dir();
        if !build.exists() {
            fs::create_dir_all(&build)?;
        }
        let mut opts = Vec::new();
        opts.push(format!(
            "-DCMAKE_INSTALL_PREFIX={}",
            self.prefix().display()
        ));
        if let Some(ref option) = self.option {
            if let Some(ref target) = option.target {
                opts.push(format!(
                    "-DLLVM_TARGETS_TO_BUILD={}",
                    target.iter().join(";")
                ));
            }
            if let Some(ref example) = option.example {
                let ex = if *example { 1 } else { 0 };
                opts.push(format!("-DLLVM_INCLUDE_EXAMPLES={}", ex));
                opts.push(format!("-DCLANG_INCLUDE_EXAMPLES={}", ex));
            }
            if let Some(ref document) = option.example {
                let ex = if *document { 1 } else { 0 };
                opts.push(format!("-DLLVM_INCLUDE_DOCS={}", ex));
                opts.push(format!("-DCLANG_INCLUDE_DOCS={}", ex));
            }
            opts.push(format!("-DLLVM_INCLUDE_TEST=0"));
            opts.push(format!("-DCLANG_INCLUDE_TEST=0"));
        }
        process::Command::new("cmake")
            .args(&opts)
            .arg(self.src_dir())
            .current_dir(&build)
            .check_run()?;

        process::Command::new("make")
            .arg(format!("-j{}", nproc))
            .current_dir(&build)
            .check_run()?;

        process::Command::new("make")
            .arg("install")
            .current_dir(&build)
            .check_run()?;
        Ok(())
    }
}
