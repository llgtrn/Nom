use std::hash::{Hash, Hasher, DefaultHasher};

pub fn fingerprint_inputs(
    class_type: &str,
    inputs: &[(String, String)],
    ancestors: &[u64],
) -> u64 {
    let mut h = DefaultHasher::new();
    class_type.hash(&mut h);
    for (name, val) in inputs {
        name.hash(&mut h);
        val.hash(&mut h);
    }
    for &a in ancestors {
        a.hash(&mut h);
    }
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_inputs_same_hash() {
        let inputs = vec![("text".to_string(), "hello".to_string())];
        let a = fingerprint_inputs("CLIPTextEncode", &inputs, &[10, 20]);
        let b = fingerprint_inputs("CLIPTextEncode", &inputs, &[10, 20]);
        assert_eq!(a, b);
    }

    #[test]
    fn changed_input_different_hash() {
        let inputs_a = vec![("text".to_string(), "hello".to_string())];
        let inputs_b = vec![("text".to_string(), "world".to_string())];
        let a = fingerprint_inputs("CLIPTextEncode", &inputs_a, &[]);
        let b = fingerprint_inputs("CLIPTextEncode", &inputs_b, &[]);
        assert_ne!(a, b);
    }

    #[test]
    fn ancestor_change_propagates() {
        let inputs = vec![("k".to_string(), "v".to_string())];
        let a = fingerprint_inputs("Node", &inputs, &[1]);
        let b = fingerprint_inputs("Node", &inputs, &[2]);
        assert_ne!(a, b);
    }

    #[test]
    fn empty_inputs_deterministic() {
        let a = fingerprint_inputs("Empty", &[], &[]);
        let b = fingerprint_inputs("Empty", &[], &[]);
        assert_eq!(a, b);
    }
}
