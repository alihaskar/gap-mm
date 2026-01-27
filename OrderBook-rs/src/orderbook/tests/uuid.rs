#[cfg(test)]
mod tests {
    use pricelevel::UuidGenerator;
    use std::collections::HashSet;
    use std::sync::{Arc, Barrier};
    use std::thread;
    use uuid::Uuid;

    #[test]
    fn test_uuid_generator_creates_unique_ids() {
        let namespace = Uuid::new_v4();
        let generator = UuidGenerator::new(namespace);

        let id1 = generator.next();
        let id2 = generator.next();

        assert_ne!(id1, id2, "Sequential UUIDs should be different");
    }

    #[test]
    fn test_uuid_generator_is_deterministic() {
        // With the same namespace and sequence, should generate the same UUIDs
        let namespace = Uuid::parse_str("6ba7b810-9dad-11d1-80b4-00c04fd430c8").unwrap();

        let generator1 = UuidGenerator::new(namespace);
        let generator2 = UuidGenerator::new(namespace);

        assert_eq!(
            generator1.next(),
            generator2.next(),
            "First UUIDs should match with same namespace"
        );
        assert_eq!(
            generator1.next(),
            generator2.next(),
            "Second UUIDs should match with same namespace"
        );
    }

    #[test]
    fn test_different_namespaces_generate_different_uuids() {
        let namespace1 = Uuid::new_v4();
        let namespace2 = Uuid::new_v4();

        let generator1 = UuidGenerator::new(namespace1);
        let generator2 = UuidGenerator::new(namespace2);

        assert_ne!(
            generator1.next(),
            generator2.next(),
            "UUIDs from different namespaces should differ"
        );
    }

    #[test]
    fn test_uuid_generator_thread_safety() {
        let namespace = Uuid::new_v4();
        let generator = Arc::new(UuidGenerator::new(namespace));

        let num_threads = 4;
        let ids_per_thread = 25;
        let total_ids = num_threads * ids_per_thread;

        let barrier = Arc::new(Barrier::new(num_threads));
        let all_ids = Arc::new(std::sync::Mutex::new(Vec::with_capacity(total_ids)));

        let mut handles = vec![];

        for _ in 0..num_threads {
            let thread_generator = Arc::clone(&generator);
            let thread_barrier = Arc::clone(&barrier);
            let thread_ids = Arc::clone(&all_ids);

            let handle = thread::spawn(move || {
                // Wait for all threads to be ready
                thread_barrier.wait();

                let mut local_ids = Vec::with_capacity(ids_per_thread);
                for _ in 0..ids_per_thread {
                    local_ids.push(thread_generator.next());
                }

                // Add to shared collection
                let mut all = thread_ids.lock().unwrap();
                all.extend(local_ids);
            });

            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Check that all UUIDs are unique
        let all_ids = all_ids.lock().unwrap();
        let unique_ids: HashSet<_> = all_ids.iter().collect();

        assert_eq!(
            unique_ids.len(),
            total_ids,
            "All generated UUIDs should be unique"
        );
    }

    #[test]
    fn test_uuid_version_and_variant() {
        let namespace = Uuid::new_v4();
        let generator = UuidGenerator::new(namespace);

        let id = generator.next();

        // Check UUID version (should be v5, SHA1-based)
        assert_eq!(
            id.get_version(),
            Some(uuid::Version::Sha1),
            "Should be a v5 UUID"
        );

        // Variant should be RFC4122
        assert_eq!(
            id.get_variant(),
            uuid::Variant::RFC4122,
            "Should be RFC4122 variant"
        );
    }
}
