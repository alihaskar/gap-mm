#[cfg(test)]
mod tests {
    use pricelevel::TimeInForce;
    use std::str::FromStr;

    #[test]
    fn test_time_in_force_immediate_property() {
        assert!(
            !TimeInForce::Gtc.is_immediate(),
            "GTC should not be immediate"
        );
        assert!(TimeInForce::Ioc.is_immediate(), "IOC should be immediate");
        assert!(TimeInForce::Fok.is_immediate(), "FOK should be immediate");
        assert!(
            !TimeInForce::Gtd(1000).is_immediate(),
            "GTD should not be immediate"
        );
        assert!(
            !TimeInForce::Day.is_immediate(),
            "DAY should not be immediate"
        );
    }

    #[test]
    fn test_time_in_force_has_expiry_property() {
        assert!(!TimeInForce::Gtc.has_expiry(), "GTC should not have expiry");
        assert!(!TimeInForce::Ioc.has_expiry(), "IOC should not have expiry");
        assert!(!TimeInForce::Fok.has_expiry(), "FOK should not have expiry");
        assert!(
            TimeInForce::Gtd(1000).has_expiry(),
            "GTD should have expiry"
        );
        assert!(TimeInForce::Day.has_expiry(), "DAY should have expiry");
    }

    #[test]
    fn test_time_in_force_is_expired() {
        let current_time = 1000;
        let market_close = Some(1500);

        // GTD expiry tests
        let gtd_past = TimeInForce::Gtd(900);
        let gtd_future = TimeInForce::Gtd(1100);

        assert!(
            gtd_past.is_expired(current_time, market_close),
            "GTD with past timestamp should be expired"
        );
        assert!(
            !gtd_future.is_expired(current_time, market_close),
            "GTD with future timestamp should not be expired"
        );

        // DAY expiry tests
        let day = TimeInForce::Day;

        assert!(
            !day.is_expired(current_time, market_close),
            "DAY before market close should not be expired"
        );
        assert!(
            day.is_expired(1600, market_close),
            "DAY after market close should be expired"
        );
        assert!(
            !day.is_expired(current_time, None),
            "DAY without market close should not be expired"
        );

        // Non-expirable types
        assert!(
            !TimeInForce::Gtc.is_expired(current_time, market_close),
            "GTC should never expire"
        );
        assert!(
            !TimeInForce::Ioc.is_expired(current_time, market_close),
            "IOC should not expire (it's immediate)"
        );
        assert!(
            !TimeInForce::Fok.is_expired(current_time, market_close),
            "FOK should not expire (it's immediate)"
        );
    }

    #[test]
    fn test_time_in_force_display() {
        assert_eq!(format!("{}", TimeInForce::Gtc), "GTC");
        assert_eq!(format!("{}", TimeInForce::Ioc), "IOC");
        assert_eq!(format!("{}", TimeInForce::Fok), "FOK");
        assert_eq!(format!("{}", TimeInForce::Gtd(12345)), "GTD-12345");
        assert_eq!(format!("{}", TimeInForce::Day), "DAY");
    }

    #[test]
    fn test_time_in_force_from_str() {
        assert_eq!(TimeInForce::from_str("GTC").unwrap(), TimeInForce::Gtc);
        assert_eq!(TimeInForce::from_str("IOC").unwrap(), TimeInForce::Ioc);
        assert_eq!(TimeInForce::from_str("FOK").unwrap(), TimeInForce::Fok);
        assert_eq!(TimeInForce::from_str("DAY").unwrap(), TimeInForce::Day);

        // Case insensitivity
        assert_eq!(TimeInForce::from_str("gtc").unwrap(), TimeInForce::Gtc);
        assert_eq!(TimeInForce::from_str("ioc").unwrap(), TimeInForce::Ioc);
        assert_eq!(TimeInForce::from_str("fok").unwrap(), TimeInForce::Fok);
        assert_eq!(TimeInForce::from_str("day").unwrap(), TimeInForce::Day);

        // GTD with timestamp
        assert_eq!(
            TimeInForce::from_str("GTD-12345").unwrap(),
            TimeInForce::Gtd(12345)
        );

        // Invalid formats
        assert!(TimeInForce::from_str("INVALID").is_err());
        assert!(TimeInForce::from_str("GTD").is_err());
        assert!(TimeInForce::from_str("GTD-ABC").is_err());
    }
}
