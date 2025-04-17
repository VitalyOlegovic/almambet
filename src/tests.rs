#[cfg(test)]
mod tests {
    
    use crate::spam_filter::check_message_spam;
    use crate::spam_filter::spam_filter_settings::SpamFilterSettings;
    use crate::mail_reader::message::Message;
    
    #[test]
    fn test_spam_filter_matches_domain() {
        let settings = SpamFilterSettings {
            from_regular_expressions: vec![r"nuovapromo\\.it$".to_string()],
            ..Default::default()
        };
        
        let spam_message = Message {
            from: "support@nw.nuovapromo.it".to_string(),
            ..Default::default()
        };
        
        assert!(check_message_spam(&spam_message, &settings));
    }
    
    #[test]
    fn test_spam_filter_ignores_good_email() {
        let settings = SpamFilterSettings {
            from_regular_expressions: vec![r"maildelgiorno\.it$".to_string()],
            ..Default::default()
        };
        
        let good_message = Message {
            from: "friend@gmail.com".to_string(),
            ..Default::default()
        };
        
        assert!(!check_message_spam(&good_message, &settings));
    }
}