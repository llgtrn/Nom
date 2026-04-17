use crate::span::TraceId;

pub enum Sampler {
    AlwaysOn,
    AlwaysOff,
    TraceIdRatio { ratio: f64 },
    ParentBased { root: Box<Sampler> },
}

impl Sampler {
    pub fn should_sample(&self, parent_sampled: Option<bool>, trace_id: &TraceId) -> bool {
        match self {
            Self::AlwaysOn => true,
            Self::AlwaysOff => false,
            Self::TraceIdRatio { ratio } => {
                let mut bytes = [0u8; 8];
                bytes.copy_from_slice(&trace_id[..8]);
                let v = u64::from_le_bytes(bytes);
                // ratio 0.0 → never sample; ratio 1.0 → always sample
                if *ratio <= 0.0 {
                    return false;
                }
                if *ratio >= 1.0 {
                    return true;
                }
                let threshold = (u64::MAX as f64 * ratio) as u64;
                v <= threshold
            }
            Self::ParentBased { root } => match parent_sampled {
                Some(sampled) => sampled,
                None => root.should_sample(None, trace_id),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trace(seed: u64) -> TraceId {
        let mut id = [0u8; 16];
        id[..8].copy_from_slice(&seed.to_le_bytes());
        id
    }

    #[test]
    fn always_on() {
        let s = Sampler::AlwaysOn;
        assert!(s.should_sample(None, &trace(0)));
        assert!(s.should_sample(Some(false), &trace(0)));
    }

    #[test]
    fn always_off() {
        let s = Sampler::AlwaysOff;
        assert!(!s.should_sample(None, &trace(u64::MAX)));
        assert!(!s.should_sample(Some(true), &trace(u64::MAX)));
    }

    #[test]
    fn ratio_zero_never_samples() {
        let s = Sampler::TraceIdRatio { ratio: 0.0 };
        assert!(!s.should_sample(None, &trace(0)));
        assert!(!s.should_sample(None, &trace(u64::MAX)));
    }

    #[test]
    fn ratio_one_always_samples() {
        let s = Sampler::TraceIdRatio { ratio: 1.0 };
        assert!(s.should_sample(None, &trace(0)));
        assert!(s.should_sample(None, &trace(u64::MAX)));
    }

    #[test]
    fn ratio_half_approximately_half() {
        let s = Sampler::TraceIdRatio { ratio: 0.5 };
        let sampled: usize = (0u64..1000)
            .filter(|&i| s.should_sample(None, &trace(i * 18446744073709552)))
            .count();
        assert!(sampled > 300 && sampled < 700, "sampled={sampled}");
    }

    #[test]
    fn parent_based_honors_some_true() {
        let s = Sampler::ParentBased { root: Box::new(Sampler::AlwaysOff) };
        assert!(s.should_sample(Some(true), &trace(0)));
    }

    #[test]
    fn parent_based_honors_some_false() {
        let s = Sampler::ParentBased { root: Box::new(Sampler::AlwaysOn) };
        assert!(!s.should_sample(Some(false), &trace(0)));
    }
}
