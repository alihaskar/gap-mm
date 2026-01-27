#[cfg(test)]
mod tests {
    use crate::OrderBookError;
    use pricelevel::{PriceLevelError, Side};

    #[test]
    fn test_display_price_level_error() {
        let err = OrderBookError::PriceLevelError(PriceLevelError::InvalidFormat);
        assert_eq!(format!("{err}"), "Price level error: Invalid format");
    }

    #[test]
    fn test_display_order_not_found() {
        let order_id = "e4968197-6137-47a4-ba79-690d8c552248";
        let err = OrderBookError::OrderNotFound(order_id.to_string());
        assert_eq!(format!("{err}"), format!("Order not found: {}", order_id));
    }

    #[test]
    fn test_display_invalid_price_level() {
        let price = 1000;
        let err = OrderBookError::InvalidPriceLevel(price);
        assert_eq!(format!("{err}"), format!("Invalid price level: {}", price));
    }

    #[test]
    fn test_display_price_crossing() {
        let err = OrderBookError::PriceCrossing {
            price: 1000,
            side: Side::Buy,
            opposite_price: 999,
        };
        assert_eq!(
            format!("{err}"),
            "Price crossing: BUY 1000 would cross opposite at 999"
        );
    }

    #[test]
    fn test_display_insufficient_liquidity() {
        let err = OrderBookError::InsufficientLiquidity {
            side: Side::Sell,
            requested: 100,
            available: 50,
        };
        assert_eq!(
            format!("{err}"),
            "Insufficient liquidity for SELL order: requested 100, available 50"
        );
    }

    #[test]
    fn test_display_invalid_operation() {
        let message = "Cannot update price to the same value";
        let err = OrderBookError::InvalidOperation {
            message: message.to_string(),
        };
        assert_eq!(format!("{err}"), format!("Invalid operation: {}", message));
    }

    #[test]
    fn test_from_price_level_error() {
        let price_level_error = PriceLevelError::InvalidFormat;
        let order_book_error: OrderBookError = price_level_error.into();

        match order_book_error {
            OrderBookError::PriceLevelError(err) => match err {
                PriceLevelError::InvalidFormat => (),
                _ => panic!("Expected PriceLevelError::InvalidFormat"),
            },
            _ => panic!("Expected OrderBookError::PriceLevelError"),
        }
    }

    #[test]
    fn test_error_trait_implementation() {
        let err = OrderBookError::InvalidPriceLevel(1000);
        let _: &dyn std::error::Error = &err; // This will compile only if OrderBookError implements std::error::Error
    }

    #[test]
    fn test_missing_field_conversion() {
        let field_name = "price";
        let price_level_error = PriceLevelError::MissingField(field_name.to_string());
        let order_book_error: OrderBookError = price_level_error.into();

        match order_book_error {
            OrderBookError::PriceLevelError(PriceLevelError::MissingField(field)) => {
                assert_eq!(field, field_name);
            }
            _ => panic!("Expected OrderBookError::PriceLevelError(PriceLevelError::MissingField)"),
        }
    }

    #[test]
    fn test_invalid_field_value_conversion() {
        let price_level_error = PriceLevelError::InvalidFieldValue {
            field: "price".to_string(),
            value: "invalid".to_string(),
        };
        let order_book_error: OrderBookError = price_level_error.into();

        match order_book_error {
            OrderBookError::PriceLevelError(PriceLevelError::InvalidFieldValue {
                field,
                value,
            }) => {
                assert_eq!(field, "price");
                assert_eq!(value, "invalid");
            }
            _ => panic!(
                "Expected OrderBookError::PriceLevelError(PriceLevelError::InvalidFieldValue)"
            ),
        }
    }
}
