use super::ConfigError;

#[derive(Clone, Debug)]
pub struct RabbitMqConfig {
    pub rabbitmq_host: String,
    pub rabbitmq_port: u16,
    pub rabbitmq_user: String,
    pub rabbitmq_password: String,
    pub rabbitmq_exchange: String,
    pub redirect_event_routing_key: String,
}

impl RabbitMqConfig {
    pub fn amqp_url(&self) -> Result<String, ConfigError> {
        let mut rabbitmq_url = url::Url::parse(
            format!("amqp://{}:{}/%2F", self.rabbitmq_host, self.rabbitmq_port).as_mut_str(),
        )
        .map_err(|_e| {
            ConfigError::Url("Failed to parse url from rabbitmq configuration".to_string())
        })?;
        rabbitmq_url
            .set_username(&self.rabbitmq_user)
            .map_err(|_e| ConfigError::Url("Failed to set amqp user".to_string()))?;
        rabbitmq_url
            .set_password(Some(&self.rabbitmq_password))
            .map_err(|_e| ConfigError::Url("Failed to set amqp password".to_string()))?;

        Ok(rabbitmq_url.to_string())
    }
}
