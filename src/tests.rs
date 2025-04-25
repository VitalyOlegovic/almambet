#[cfg(test)]
mod tests {
    
    use crate::mail_move_rules::check_message_matches;
    use crate::mail_move_rules::mail_move_settings::Rule;
    use crate::mail_reader::message::Message;
    
    #[test]
    fn test_mail_mover_matches_domain() {
        let settings = Rule {
            from: Some(vec![r"nuovapromo\\.it$".to_string()]),
            ..Default::default()
        };
        
        let spam_message = Message {
            from: "support@nw.nuovapromo.it".to_string(),
            ..Default::default()
        };
        
        assert!(check_message_matches(&spam_message, &settings));
    }
    
    #[test]
    fn test_spam_filter_ignores_good_email() {
        let settings = Rule {
            from: Some(vec![r"maildelgiorno\.it$".to_string()]),
            ..Default::default()
        };
        
        let good_message = Message {
            from: "friend@gmail.com".to_string(),
            ..Default::default()
        };
        
        assert!(!check_message_matches(&good_message, &settings));
    }
}