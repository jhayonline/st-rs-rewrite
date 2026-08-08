use std::env;

pub mod database;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub jwt_expiration: u64,
    pub server_host: String,
    pub server_port: u16,
    pub app_url: String,
    pub paystack_secret_key: Option<String>,
    pub brevo_api_key: Option<String>,
    pub brevo_sender_email: Option<String>,
    pub brevo_sender_name: Option<String>,
    pub cloudinary_cloud_name: Option<String>,
    pub cloudinary_api_key: Option<String>,
    pub cloudinary_api_secret: Option<String>,
}

impl Config {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        Self {
            database_url: env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            jwt_secret: env::var("JWT_SECRET").expect("JWT_SECRET must be set"),
            jwt_expiration: env::var("JWT_EXPIRATION")
                .unwrap_or_else(|_| "604800".to_string())
                .parse()
                .expect("JWT_EXPIRATION must be a number"),
            server_host: env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            server_port: env::var("SERVER_PORT")
                .unwrap_or_else(|_| "8698".to_string())
                .parse()
                .expect("SERVER_PORT must be a number"),
            app_url: env::var("APP_URL").unwrap_or_else(|_| "http://localhost:8698".to_string()),
            paystack_secret_key: env::var("PAYSTACK_SECRET_KEY").ok(),
            brevo_api_key: env::var("BREVO_API_KEY").ok(),
            brevo_sender_email: env::var("BREVO_SENDER_EMAIL").ok(),
            brevo_sender_name: env::var("BREVO_SENDER_NAME").ok(),
            cloudinary_cloud_name: env::var("CLOUDINARY_CLOUD_NAME").ok(),
            cloudinary_api_key: env::var("CLOUDINARY_API_KEY").ok(),
            cloudinary_api_secret: env::var("CLOUDINARY_API_SECRET").ok(),
        }
    }
}
