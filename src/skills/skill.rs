use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
pub struct SkillFrontmatter {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub user_invocable: bool,
    #[serde(default)]
    pub disable_model_invocation: bool,
}

#[derive(Clone)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub instructions: String,
    pub path: PathBuf,
    pub user_invocable: bool,
    pub disable_model_invocation: bool,
}

pub fn load_all_skills() -> Vec<Skill> {
    let mut skills = Vec::new();

    // Project skills (folder 'skills/' in the root directory) 
    skills.extend(load_skills_from_dir(&PathBuf::from("skills")));

    // Global user skils
    if let Some(config_dir) = dirs::config_dir() {
        skills.extend(load_skills_from_dir(&config_dir.join("nini/skills")));
    }

    // Remove duplicates (project skill takes priority)
    skills.sort_by(|a, b| a.name.cmp(&b.name));
    skills.dedup_by(|a, b| a.name == b.name);

    skills
}

pub fn load_skills_from_dir(dir: &Path) -> Vec<Skill> {
    let mut skills = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let skill_file = path.join("SKILL.md");
                if skill_file.exists() {
                    if let Ok(content) = std::fs::read_to_string(&skill_file) {
                        if let Some(skill) = parse_skill(&content, path) {
                            skills.push(skill);
                        }
                    }
                }
            }
        }
    }
    skills
}

fn parse_skill(content: &str, path: PathBuf) -> Option<Skill> {
    let parts: Vec<&str> = content.splitn(3, "---").collect();
    if parts.len() < 3  {
        eprintln!("Skill in {:?} does not have a valid frontmatter", path);
        return None;
    }

    let frontmatter: SkillFrontmatter = match serde_yaml::from_str(parts[1]) {
        Ok(fm) => fm,
        Err(e) => {
            eprintln!("Error parsing frontmatter of {:?}: {}", path, e);
            return None;
        }
    };

    Some(Skill {
        name: frontmatter.name,
        description: frontmatter.description,
        instructions: parts[2].trim().to_string(),
        path,
        user_invocable: frontmatter.user_invocable,
        disable_model_invocation: frontmatter.disable_model_invocation,
    })
}

impl Skill {
    pub fn to_prompt(&self) -> String {
        format!(
            "## Skill: {}\n\n**Description:** {}\n\n**Instructions:**\n{}\n",
            self.name, self.description, self.instructions
        )
    }

    pub fn is_user_invocable(&self) -> bool {
        self.user_invocable
    }
}
