use self::WriterMode::*;
use crate::commit::{Commit, CommitType};
use crate::COMMITS_METADATA;
use anyhow::Result;
use git2::Oid;
use std::fs;
use std::path::PathBuf;

pub enum WriterMode {
    Replace,
    Prepend,
    Append,
}

pub(crate) struct Changelog {
    pub from: Oid,
    pub to: Oid,
    pub date: String,
    pub commits: Vec<Commit>,
    pub tag_name: Option<String>,
}

pub(crate) struct ChangelogWriter {
    pub(crate) changelog: Changelog,
    pub(crate) path: PathBuf,
    pub(crate) mode: WriterMode,
}

impl ChangelogWriter {
    pub(crate) fn write(&mut self) -> Result<()> {
        match &self.mode {
            Append => self.insert(),
            Prepend => self.insert(),
            Replace => self.replace(),
        }
    }

    fn insert(&mut self) -> Result<()> {
        let mut changelog_content =
            fs::read_to_string(&self.path).unwrap_or_else(|_err| Changelog::changelog_template());

        let separator_idx = match self.mode {
            Append => changelog_content.rfind("- - -"),
            Prepend => changelog_content.find("- - -"),
            _ => unreachable!(),
        };

        if let Some(idx) = separator_idx {
            let markdown_changelog = self.changelog.markdown(false);
            changelog_content.insert_str(idx + 5, &markdown_changelog);
            changelog_content.insert_str(idx + 5 + markdown_changelog.len(), "\n- - -");
            fs::write(&self.path, changelog_content)?;

            Ok(())
        } else {
            Err(anyhow!(
                "Cannot find default separator '- - -' in {}",
                self.path.display()
            ))
        }
    }

    fn replace(&mut self) -> Result<()> {
        let mut content = Changelog::default_header();
        content.push_str(&self.changelog.markdown(false));
        content.push_str(Changelog::default_footer().as_str());

        fs::write(&self.path, content).map_err(|err| anyhow!(err))
    }
}

impl Changelog {
    pub(crate) fn markdown(&mut self, colored: bool) -> String {
        let mut out = String::new();

        let short_to = &self.to.to_string()[0..6];
        let short_from = &self.from.to_string()[0..6];
        let version_title = self
            .tag_name
            .as_ref()
            .cloned()
            .unwrap_or(format!("{}..{}", short_from, short_to));

        out.push_str(&format!("\n## {} - {}\n\n", version_title, self.date));

        let add_commit_section = |commit_type: &CommitType| {
            let commits: Vec<Commit> = self
                .commits
                .drain_filter(|commit| &commit.message.commit_type == commit_type)
                .collect();

            let metadata = COMMITS_METADATA.get(&commit_type).unwrap();
            if !commits.is_empty() {
                out.push_str(&format!("\n### {}\n\n", metadata.changelog_title));

                commits.iter().for_each(|commit| {
                    out.push_str(&commit.to_markdown(colored));
                });
            }
        };

        COMMITS_METADATA
            .iter()
            .map(|(commit_type, _)| commit_type)
            .for_each(add_commit_section);

        out
    }

    pub(crate) fn default_header() -> String {
        let title = "# Changelog";
        let link = "[conventional commits]";
        format!(
            "{}\nAll notable changes to this project will be documented in this file. \
        See {}(https://www.conventionalcommits.org/) for commit guidelines.\n\n- - -\n",
            title, link
        )
    }

    pub(crate) fn default_footer() -> String {
        "\nThis changelog was generated by [cocogitto](https://github.com/oknozor/cocogitto)."
            .to_string()
    }

    fn changelog_template() -> String {
        let mut content = Changelog::default_header();
        content.push_str(&Changelog::default_footer());
        content
    }
}

#[cfg(test)]
mod test {
    use crate::changelog::Changelog;
    use crate::commit::{Commit, CommitMessage, CommitType};
    use anyhow::Result;
    use chrono::Utc;
    use git2::Oid;

    #[test]
    fn should_generate_changelog() -> Result<()> {
        // Arrange
        let mut ch = Changelog {
            from: Oid::from_str("5375e15770ddf8821d0c1ad393d315e243014c15")?,
            to: Oid::from_str("35085f20c5293fc8830e4e44a9bb487f98734f73")?,
            date: Utc::now().date().naive_local().to_string(),
            tag_name: None,
            commits: vec![
                Commit {
                    oid: "5375e15770ddf8821d0c1ad393d315e243014c15".to_string(),
                    message: CommitMessage {
                        commit_type: CommitType::Feature,
                        scope: None,
                        body: None,
                        footer: None,
                        description: "this is a commit message".to_string(),
                        is_breaking_change: false,
                    },
                    author: "coco".to_string(),
                    date: Utc::now().naive_local(),
                },
                Commit {
                    oid: "5375e15770ddf8821d0c1ad393d315e243014c15".to_string(),
                    message: CommitMessage {
                        commit_type: CommitType::Feature,
                        scope: None,
                        body: None,
                        footer: None,
                        description: "this is an other commit message".to_string(),
                        is_breaking_change: false,
                    },
                    author: "cogi".to_string(),
                    date: Utc::now().naive_local(),
                },
            ],
        };

        // Act
        let content = ch.markdown(false);

        // Assert
        println!("{}", content);
        assert!(content.contains(
            "[5375e1](https://github.com/oknozor/cocogitto/commit/5375e15770ddf8821d0c1ad393d315e243014c15) - this is a commit message - coco"
        ));
        assert!(content.contains(
            "[5375e1](https://github.com/oknozor/cocogitto/commit/5375e15770ddf8821d0c1ad393d315e243014c15) - this is an other commit message - cogi"
        ));
        assert!(content.contains("## 5375e1..35085f -"));
        assert!(content.contains("### Features"));
        assert!(!content.contains("### Tests"));
        Ok(())
    }

    #[test]
    fn should_generate_empty_changelog() -> Result<()> {
        // Arrange
        let mut ch = Changelog {
            from: Oid::from_str("5375e15770ddf8821d0c1ad393d315e243014c15")?,
            to: Oid::from_str("35085f20c5293fc8830e4e44a9bb487f98734f73")?,
            date: Utc::now().date().naive_local().to_string(),
            commits: vec![],
            tag_name: None,
        };

        // Act
        let content = ch.markdown(false);

        // Assert
        println!("{}", content);
        assert!(content.contains("## 5375e1..35085f"));
        assert!(!content.contains("### Features"));
        Ok(())
    }
}
