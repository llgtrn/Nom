/// Mobile platform targets.
#[derive(Debug, Clone, PartialEq)]
pub enum MobilePlatform {
    Ios,
    Android,
    CrossPlatform,
}

impl MobilePlatform {
    pub fn platform_name(&self) -> &str {
        match self {
            MobilePlatform::Ios => "ios",
            MobilePlatform::Android => "android",
            MobilePlatform::CrossPlatform => "cross-platform",
        }
    }
}

/// A UI component within a mobile screen.
#[derive(Debug, Clone, PartialEq)]
pub enum MobileComponent {
    Header(String),
    List,
    Form,
    Button(String),
    Image,
}

impl MobileComponent {
    pub fn component_name(&self) -> &str {
        match self {
            MobileComponent::Header(_) => "header",
            MobileComponent::List => "list",
            MobileComponent::Form => "form",
            MobileComponent::Button(_) => "button",
            MobileComponent::Image => "image",
        }
    }
}

/// A single screen in a mobile app.
#[derive(Debug, Clone)]
pub struct MobileScreen {
    pub name: String,
    pub route: String,
    pub components: Vec<MobileComponent>,
}

impl MobileScreen {
    pub fn new(name: impl Into<String>, route: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            route: route.into(),
            components: Vec::new(),
        }
    }

    pub fn add_component(&mut self, c: MobileComponent) {
        self.components.push(c);
    }

    pub fn component_count(&self) -> usize {
        self.components.len()
    }

    /// Returns true if any component is a `Form`.
    pub fn has_form(&self) -> bool {
        self.components.iter().any(|c| matches!(c, MobileComponent::Form))
    }
}

/// Full specification of a mobile app.
#[derive(Debug, Clone)]
pub struct MobileAppSpec {
    pub app_name: String,
    pub platform: MobilePlatform,
    pub screens: Vec<MobileScreen>,
}

impl MobileAppSpec {
    pub fn new(app_name: impl Into<String>, platform: MobilePlatform) -> Self {
        Self {
            app_name: app_name.into(),
            platform,
            screens: Vec::new(),
        }
    }

    pub fn add_screen(&mut self, screen: MobileScreen) {
        self.screens.push(screen);
    }

    pub fn screen_count(&self) -> usize {
        self.screens.len()
    }

    /// Sum of component counts across all screens.
    pub fn total_components(&self) -> usize {
        self.screens.iter().map(|s| s.component_count()).sum()
    }
}

/// Composes mobile apps from natural-language intent.
pub struct MobileComposer {
    pub platform: MobilePlatform,
}

impl MobileComposer {
    pub fn new(platform: MobilePlatform) -> Self {
        Self { platform }
    }

    /// Build a `MobileAppSpec` from intent text.
    ///
    /// Rules:
    /// - Always creates a Home screen with `Header` + `List`.
    /// - If "login" appears in intent → adds Login screen with `Form` + `Button`.
    /// - If "profile" appears in intent → adds Profile screen with `Image` + `Form`.
    pub fn compose_from_intent(&self, app_name: &str, intent: &str) -> MobileAppSpec {
        let mut spec = MobileAppSpec::new(app_name, self.platform.clone());

        // Home screen — always present
        let mut home = MobileScreen::new("Home", "/home");
        home.add_component(MobileComponent::Header("Home".to_string()));
        home.add_component(MobileComponent::List);
        spec.add_screen(home);

        if intent.contains("login") {
            let mut login = MobileScreen::new("Login", "/login");
            login.add_component(MobileComponent::Form);
            login.add_component(MobileComponent::Button("Sign In".to_string()));
            spec.add_screen(login);
        }

        if intent.contains("profile") {
            let mut profile = MobileScreen::new("Profile", "/profile");
            profile.add_component(MobileComponent::Image);
            profile.add_component(MobileComponent::Form);
            spec.add_screen(profile);
        }

        spec
    }

    /// Estimate number of screens: 1 base + 1 per "login" + 1 per "profile".
    pub fn screen_count_estimate(&self, intent: &str) -> usize {
        let mut count = 1usize;
        if intent.contains("login") {
            count += 1;
        }
        if intent.contains("profile") {
            count += 1;
        }
        count
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod mobile_compose_tests {
    use super::*;

    #[test]
    fn mobile_platform_platform_name() {
        assert_eq!(MobilePlatform::Ios.platform_name(), "ios");
        assert_eq!(MobilePlatform::Android.platform_name(), "android");
        assert_eq!(MobilePlatform::CrossPlatform.platform_name(), "cross-platform");
    }

    #[test]
    fn mobile_component_component_name() {
        assert_eq!(MobileComponent::Header("Title".to_string()).component_name(), "header");
        assert_eq!(MobileComponent::List.component_name(), "list");
        assert_eq!(MobileComponent::Form.component_name(), "form");
        assert_eq!(MobileComponent::Button("OK".to_string()).component_name(), "button");
        assert_eq!(MobileComponent::Image.component_name(), "image");
    }

    #[test]
    fn mobile_screen_add_and_count() {
        let mut screen = MobileScreen::new("Home", "/home");
        assert_eq!(screen.component_count(), 0);
        screen.add_component(MobileComponent::Header("Home".to_string()));
        screen.add_component(MobileComponent::List);
        assert_eq!(screen.component_count(), 2);
    }

    #[test]
    fn mobile_screen_has_form_true() {
        let mut screen = MobileScreen::new("Login", "/login");
        screen.add_component(MobileComponent::Form);
        assert!(screen.has_form());
    }

    #[test]
    fn mobile_screen_has_form_false() {
        let mut screen = MobileScreen::new("Home", "/home");
        screen.add_component(MobileComponent::List);
        assert!(!screen.has_form());
    }

    #[test]
    fn mobile_app_spec_total_components() {
        let mut spec = MobileAppSpec::new("MyApp", MobilePlatform::CrossPlatform);

        let mut s1 = MobileScreen::new("Home", "/home");
        s1.add_component(MobileComponent::Header("Home".to_string()));
        s1.add_component(MobileComponent::List);

        let mut s2 = MobileScreen::new("Login", "/login");
        s2.add_component(MobileComponent::Form);
        s2.add_component(MobileComponent::Button("Sign In".to_string()));

        spec.add_screen(s1);
        spec.add_screen(s2);

        assert_eq!(spec.screen_count(), 2);
        assert_eq!(spec.total_components(), 4);
    }

    #[test]
    fn mobile_composer_compose_base_screen() {
        let composer = MobileComposer::new(MobilePlatform::Ios);
        let spec = composer.compose_from_intent("SimpleApp", "show feed");
        assert_eq!(spec.screen_count(), 1);
        assert_eq!(spec.screens[0].name, "Home");
        assert_eq!(spec.screens[0].component_count(), 2);
    }

    #[test]
    fn mobile_composer_compose_with_login() {
        let composer = MobileComposer::new(MobilePlatform::Android);
        let spec = composer.compose_from_intent("AuthApp", "login and browse");
        assert_eq!(spec.screen_count(), 2);
        let login = spec.screens.iter().find(|s| s.name == "Login").expect("Login screen must exist");
        assert!(login.has_form());
        assert_eq!(login.component_count(), 2);
    }

    #[test]
    fn mobile_composer_screen_count_estimate() {
        let composer = MobileComposer::new(MobilePlatform::CrossPlatform);
        assert_eq!(composer.screen_count_estimate("browse feed"), 1);
        assert_eq!(composer.screen_count_estimate("login to account"), 2);
        assert_eq!(composer.screen_count_estimate("edit profile settings"), 2);
        assert_eq!(composer.screen_count_estimate("login and edit profile"), 3);
    }
}
