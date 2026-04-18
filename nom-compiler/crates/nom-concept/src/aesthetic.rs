//! Aesthetic skill seeding infrastructure (§5.18).
//!
//! Aesthetic primitives composed via the same 3 operators → generative
//! images/audio/video/3D/typography.  Skills are seeded here and stored
//! in the dictionary at runtime.

/// Top-level creative domains supported by aesthetic skill composition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AestheticDomain {
    Web,
    GenerativeArt,
    AudioLoop,
    Video3D,
    Typography,
}

impl AestheticDomain {
    pub fn domain_name(&self) -> &str {
        match self {
            AestheticDomain::Web => "web",
            AestheticDomain::GenerativeArt => "generative_art",
            AestheticDomain::AudioLoop => "audio_loop",
            AestheticDomain::Video3D => "video_3d",
            AestheticDomain::Typography => "typography",
        }
    }

    pub fn file_extension(&self) -> &str {
        match self {
            AestheticDomain::Web => ".html",
            AestheticDomain::GenerativeArt => ".png",
            AestheticDomain::AudioLoop => ".wav",
            AestheticDomain::Video3D => ".mp4",
            AestheticDomain::Typography => ".ttf",
        }
    }
}

/// A single aesthetic composition skill.
#[derive(Debug, Clone)]
pub struct AestheticSkill {
    pub name: String,
    pub domain: AestheticDomain,
    pub description: String,
    pub nomx_template: String,
}

impl AestheticSkill {
    pub fn new(
        name: impl Into<String>,
        domain: AestheticDomain,
        description: impl Into<String>,
    ) -> Self {
        let nomx_template = format!("compose {} using", domain.domain_name());
        AestheticSkill {
            name: name.into(),
            domain,
            description: description.into(),
            nomx_template,
        }
    }
}

/// Registry of seeded aesthetic skills.
#[derive(Debug, Default)]
pub struct AestheticRegistry {
    pub skills: Vec<AestheticSkill>,
}

impl AestheticRegistry {
    pub fn new() -> Self {
        AestheticRegistry { skills: Vec::new() }
    }

    /// Seed the 9 canonical aesthetic skills from §5.18.
    pub fn seed() -> Self {
        let skills = vec![
            AestheticSkill::new(
                "compose_brutalist_webpage",
                AestheticDomain::Web,
                "Compose a brutalist-style webpage artifact",
            ),
            AestheticSkill::new(
                "compose_glass_morphism_ui",
                AestheticDomain::Web,
                "Compose a glass-morphism UI surface",
            ),
            AestheticSkill::new(
                "compose_generative_art_piece",
                AestheticDomain::GenerativeArt,
                "Compose a generative art piece as a PNG",
            ),
            AestheticSkill::new(
                "compose_particle_system",
                AestheticDomain::GenerativeArt,
                "Compose a particle-system generative image",
            ),
            AestheticSkill::new(
                "compose_lofi_audio_loop",
                AestheticDomain::AudioLoop,
                "Compose a lo-fi audio loop",
            ),
            AestheticSkill::new(
                "compose_ambient_drone",
                AestheticDomain::AudioLoop,
                "Compose an ambient drone audio loop",
            ),
            AestheticSkill::new(
                "compose_camera_fly_through",
                AestheticDomain::Video3D,
                "Compose a camera fly-through video",
            ),
            AestheticSkill::new(
                "compose_physics_sim",
                AestheticDomain::Video3D,
                "Compose a physics simulation video",
            ),
            AestheticSkill::new(
                "compose_kinetic_type",
                AestheticDomain::Typography,
                "Compose a kinetic-type typography artifact",
            ),
        ];
        AestheticRegistry { skills }
    }

    /// Return all skills belonging to `domain`.
    pub fn skills_for(&self, domain: &AestheticDomain) -> Vec<&AestheticSkill> {
        self.skills.iter().filter(|s| &s.domain == domain).collect()
    }

    pub fn skill_count(&self) -> usize {
        self.skills.len()
    }

    pub fn find_by_name(&self, name: &str) -> Option<&AestheticSkill> {
        self.skills.iter().find(|s| s.name == name)
    }
}

#[cfg(test)]
mod aesthetic_tests {
    use super::*;

    #[test]
    fn domain_name_returns_correct_string() {
        assert_eq!(AestheticDomain::Web.domain_name(), "web");
        assert_eq!(AestheticDomain::GenerativeArt.domain_name(), "generative_art");
        assert_eq!(AestheticDomain::AudioLoop.domain_name(), "audio_loop");
        assert_eq!(AestheticDomain::Video3D.domain_name(), "video_3d");
        assert_eq!(AestheticDomain::Typography.domain_name(), "typography");
    }

    #[test]
    fn file_extension_returns_correct_extension() {
        assert_eq!(AestheticDomain::Web.file_extension(), ".html");
        assert_eq!(AestheticDomain::GenerativeArt.file_extension(), ".png");
        assert_eq!(AestheticDomain::AudioLoop.file_extension(), ".wav");
        assert_eq!(AestheticDomain::Video3D.file_extension(), ".mp4");
        assert_eq!(AestheticDomain::Typography.file_extension(), ".ttf");
    }

    #[test]
    fn skill_new_sets_nomx_template() {
        let skill = AestheticSkill::new(
            "compose_brutalist_webpage",
            AestheticDomain::Web,
            "a description",
        );
        assert_eq!(skill.nomx_template, "compose web using");
    }

    #[test]
    fn seed_produces_nine_skills() {
        let registry = AestheticRegistry::seed();
        assert_eq!(registry.skills.len(), 9);
    }

    #[test]
    fn skills_for_web_returns_two() {
        let registry = AestheticRegistry::seed();
        let web_skills = registry.skills_for(&AestheticDomain::Web);
        assert_eq!(web_skills.len(), 2);
    }

    #[test]
    fn skills_for_typography_returns_one() {
        let registry = AestheticRegistry::seed();
        let typo_skills = registry.skills_for(&AestheticDomain::Typography);
        assert_eq!(typo_skills.len(), 1);
    }

    #[test]
    fn find_by_name_found() {
        let registry = AestheticRegistry::seed();
        let skill = registry.find_by_name("compose_lofi_audio_loop");
        assert!(skill.is_some());
        assert_eq!(skill.unwrap().name, "compose_lofi_audio_loop");
    }

    #[test]
    fn find_by_name_not_found() {
        let registry = AestheticRegistry::seed();
        let skill = registry.find_by_name("nonexistent_skill");
        assert!(skill.is_none());
    }

    #[test]
    fn skill_count_is_nine() {
        let registry = AestheticRegistry::seed();
        assert_eq!(registry.skill_count(), 9);
    }
}
