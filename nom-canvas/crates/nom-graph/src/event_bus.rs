/// A named topic used to categorize events on the bus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventTopic {
    pub name: String,
}

impl EventTopic {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }

    /// Returns true when both topics share the exact same name.
    pub fn matches(&self, other: &EventTopic) -> bool {
        self.name == other.name
    }

    /// Returns the topic name as a string slice.
    pub fn topic_key(&self) -> &str {
        &self.name
    }
}

/// An event published to the bus with a topic, payload size, and sequence number.
#[derive(Debug, Clone)]
pub struct BusEvent {
    pub topic: EventTopic,
    pub payload_size: usize,
    pub sequence: u64,
}

impl BusEvent {
    /// Returns true when the payload exceeds 1 KiB.
    pub fn is_large(&self) -> bool {
        self.payload_size > 1024
    }

    /// Returns a human-readable label combining the topic key and sequence number.
    pub fn event_label(&self) -> String {
        format!("{}#{}", self.topic.topic_key(), self.sequence)
    }
}

/// A subscription binding a subscriber to a specific topic.
#[derive(Debug, Clone)]
pub struct EventSubscription {
    pub topic: EventTopic,
    pub subscriber_id: u32,
    pub active: bool,
}

impl EventSubscription {
    /// Returns true when the subscription is active and the topic matches.
    pub fn is_active_for(&self, topic: &EventTopic) -> bool {
        self.active && self.topic.matches(topic)
    }

    /// Marks the subscription as inactive.
    pub fn deactivate(&mut self) {
        self.active = false;
    }
}

/// Central event bus that tracks subscriptions and counts published events.
#[derive(Debug, Default)]
pub struct EventBus {
    pub subscriptions: Vec<EventSubscription>,
    pub published: u64,
}

impl EventBus {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a new subscription on the bus.
    pub fn subscribe(&mut self, sub: EventSubscription) {
        self.subscriptions.push(sub);
    }

    /// Increments the published counter and returns the number of active
    /// subscriptions whose topic matches the event's topic.
    pub fn publish(&mut self, event: &BusEvent) -> usize {
        self.published += 1;
        self.subscriptions
            .iter()
            .filter(|s| s.is_active_for(&event.topic))
            .count()
    }

    /// Returns the total number of currently active subscriptions.
    pub fn active_subscriber_count(&self) -> usize {
        self.subscriptions.iter().filter(|s| s.active).count()
    }
}

/// Stateless helper that inspects a bus without mutating it.
pub struct EventRouter;

impl EventRouter {
    /// Returns references to all active subscriptions matching the event's topic.
    pub fn route_to_topics<'a>(bus: &'a EventBus, event: &BusEvent) -> Vec<&'a EventSubscription> {
        bus.subscriptions
            .iter()
            .filter(|s| s.is_active_for(&event.topic))
            .collect()
    }

    /// Returns the total number of events published on the bus so far.
    pub fn total_published(bus: &EventBus) -> u64 {
        bus.published
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_topic(name: &str) -> EventTopic {
        EventTopic::new(name)
    }

    fn make_event(topic: &str, payload_size: usize, sequence: u64) -> BusEvent {
        BusEvent {
            topic: make_topic(topic),
            payload_size,
            sequence,
        }
    }

    fn make_sub(topic: &str, id: u32, active: bool) -> EventSubscription {
        EventSubscription {
            topic: make_topic(topic),
            subscriber_id: id,
            active,
        }
    }

    #[test]
    fn event_topic_matches_same_name() {
        let a = make_topic("metrics");
        let b = make_topic("metrics");
        assert!(a.matches(&b));
    }

    #[test]
    fn event_topic_matches_different_name() {
        let a = make_topic("metrics");
        let b = make_topic("logs");
        assert!(!a.matches(&b));
    }

    #[test]
    fn bus_event_is_large_above_threshold() {
        let ev = make_event("x", 1025, 1);
        assert!(ev.is_large());
        let small = make_event("x", 1024, 2);
        assert!(!small.is_large());
    }

    #[test]
    fn bus_event_label_format() {
        let ev = make_event("trace", 10, 42);
        assert_eq!(ev.event_label(), "trace#42");
    }

    #[test]
    fn event_subscription_is_active_for() {
        let sub = make_sub("alerts", 1, true);
        let matching = make_topic("alerts");
        let other = make_topic("metrics");
        assert!(sub.is_active_for(&matching));
        assert!(!sub.is_active_for(&other));
    }

    #[test]
    fn event_subscription_deactivate() {
        let mut sub = make_sub("alerts", 1, true);
        sub.deactivate();
        assert!(!sub.active);
        assert!(!sub.is_active_for(&make_topic("alerts")));
    }

    #[test]
    fn event_bus_publish_returns_matching_count() {
        let mut bus = EventBus::new();
        bus.subscribe(make_sub("alerts", 1, true));
        bus.subscribe(make_sub("alerts", 2, true));
        bus.subscribe(make_sub("logs", 3, true));
        bus.subscribe(make_sub("alerts", 4, false));
        let ev = make_event("alerts", 64, 1);
        let count = bus.publish(&ev);
        assert_eq!(count, 2);
        assert_eq!(bus.published, 1);
    }

    #[test]
    fn event_bus_active_subscriber_count() {
        let mut bus = EventBus::new();
        bus.subscribe(make_sub("a", 1, true));
        bus.subscribe(make_sub("b", 2, false));
        bus.subscribe(make_sub("c", 3, true));
        assert_eq!(bus.active_subscriber_count(), 2);
    }

    #[test]
    fn event_router_route_to_topics() {
        let mut bus = EventBus::new();
        bus.subscribe(make_sub("events", 10, true));
        bus.subscribe(make_sub("events", 11, true));
        bus.subscribe(make_sub("other", 12, true));
        bus.subscribe(make_sub("events", 13, false));
        let ev = make_event("events", 8, 5);
        let routed = EventRouter::route_to_topics(&bus, &ev);
        assert_eq!(routed.len(), 2);
        assert!(routed.iter().all(|s| s.topic.name == "events" && s.active));
    }

    #[test]
    fn event_router_total_published() {
        let mut bus = EventBus::new();
        bus.subscribe(make_sub("t", 1, true));
        let ev = make_event("t", 0, 1);
        bus.publish(&ev);
        bus.publish(&ev);
        assert_eq!(EventRouter::total_published(&bus), 2);
    }
}
