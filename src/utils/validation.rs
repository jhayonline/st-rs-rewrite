use regex::Regex;
use validator::{Validate, ValidationError};

pub fn validate_email(email: &str) -> bool {
    let email_regex = Regex::new(r"^[^\s@]+@[^\s@]+\.[^\s@]+$").unwrap();
    email_regex.is_match(email)
}

pub fn validate_password(password: &str) -> Result<(), &'static str> {
    if password.len() < 8 {
        return Err("Password must be at least 8 characters long");
    }
    if password.len() > 128 {
        return Err("Password must be less than 128 characters");
    }
    Ok(())
}

pub fn validate_role(role: &str) -> bool {
    matches!(role, "Admin" | "Mentor" | "Mentee")
}

pub fn validate_status(status: &str) -> bool {
    matches!(status, "pending" | "approved" | "rejected" | "suspended")
}

pub fn validate_membership_category(category: &str) -> bool {
    matches!(category, "Student" | "Professional" | "Volunteer")
}

pub fn validate_priority(priority: &str) -> bool {
    matches!(priority, "low" | "normal" | "high" | "urgent")
}

pub fn sanitize_input(input: &str) -> String {
    input.trim().replace(['<', '>'], "")
}
