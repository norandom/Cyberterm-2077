//! No-network managed-secrets implementation for the terminal-only build.
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use warp_graphql::managed_secrets::ManagedSecret;
use warp_graphql::queries::task_secrets::ManagedSecretValue as GqlManagedSecretValue;
use warpui_core::{Entity, SingletonEntity};

use crate::ManagedSecretValue;
use crate::client::{IdentityTokenOptions, ManagedSecretsClient, SecretOwner, TaskIdentityToken};
use crate::gcp::{GcpWorkloadIdentityFederationError, GcpWorkloadIdentityFederationToken};
pub struct ManagedSecretManager<RequestScope = ()>(std::marker::PhantomData<RequestScope>);
pub trait ActorProvider: Send + Sync + 'static {
    fn actor_uid(&self) -> Option<String>;
}
pub fn convert_task_secrets(
    gql_secrets: HashMap<String, GqlManagedSecretValue>,
) -> anyhow::Result<HashMap<String, ManagedSecretValue>> {
    let mut secrets = HashMap::new();
    for (name, gql_value) in gql_secrets {
        let value = match gql_value {
            GqlManagedSecretValue::ManagedSecretRawValue(raw) => {
                ManagedSecretValue::raw_value(raw.value)
            }
            GqlManagedSecretValue::ManagedSecretAnthropicApiKeyValue(v) => {
                ManagedSecretValue::anthropic_api_key(v.api_key)
            }
            GqlManagedSecretValue::ManagedSecretAnthropicBedrockAccessKeyValue(v) => {
                ManagedSecretValue::anthropic_bedrock_access_key(
                    v.aws_access_key_id,
                    v.aws_secret_access_key,
                    v.aws_session_token,
                    v.aws_region,
                )
            }
            GqlManagedSecretValue::ManagedSecretAnthropicBedrockApiKeyValue(v) => {
                ManagedSecretValue::anthropic_bedrock_api_key(
                    v.aws_bearer_token_bedrock,
                    v.aws_region,
                )
            }
            GqlManagedSecretValue::ManagedSecretOpenAiApiKeyValue(v) => {
                ManagedSecretValue::openai_api_key(v.api_key, v.base_url)
            }
            GqlManagedSecretValue::ManagedSecretLlmProviderApiKeyValue(v) => {
                ManagedSecretValue::llm_provider_api_key(v.provider_id, v.api_key, v.base_url)
            }
            GqlManagedSecretValue::Unknown => {
                return Err(anyhow::anyhow!(
                    "Unknown secret value type for secret: {}",
                    name
                ));
            }
        };
        secrets.insert(name, value);
    }
    Ok(secrets)
}

impl<RequestScope: Send + Sync + 'static> ManagedSecretManager<RequestScope> {
    pub fn new(
        _client: Arc<dyn ManagedSecretsClient<RequestScope = RequestScope>>,
        _actor_provider: Arc<dyn ActorProvider>,
    ) -> Self {
        Self(std::marker::PhantomData)
    }
    pub fn create_secret(
        &self,
        _request_scope: RequestScope,
        _owner: SecretOwner,
        _name: String,
        _value: ManagedSecretValue,
        _description: Option<String>,
    ) -> impl Future<Output = anyhow::Result<ManagedSecret>> + use<RequestScope> {
        async {
            Err(anyhow::anyhow!(
                "managed secrets are unavailable in this product profile"
            ))
        }
    }
    pub fn delete_secret(
        &self,
        _request_scope: RequestScope,
        _owner: SecretOwner,
        _name: String,
    ) -> impl Future<Output = anyhow::Result<()>> + use<RequestScope> {
        async {
            Err(anyhow::anyhow!(
                "managed secrets are unavailable in this product profile"
            ))
        }
    }
    pub fn update_secret(
        &self,
        _request_scope: RequestScope,
        _owner: SecretOwner,
        _name: String,
        _value: Option<ManagedSecretValue>,
        _description: Option<String>,
    ) -> impl Future<Output = anyhow::Result<ManagedSecret>> + use<RequestScope> {
        async {
            Err(anyhow::anyhow!(
                "managed secrets are unavailable in this product profile"
            ))
        }
    }
    pub fn list_secrets(
        &self,
        _request_scope: Option<RequestScope>,
    ) -> impl Future<Output = anyhow::Result<Vec<ManagedSecret>>> + use<RequestScope> {
        async { Ok(Vec::new()) }
    }
    pub fn list_harness_auth_secrets(
        &self,
        _request_scope: RequestScope,
        _harness: warp_graphql::ai::AgentHarness,
    ) -> impl Future<Output = anyhow::Result<Vec<ManagedSecret>>> + use<RequestScope> {
        async { Ok(Vec::new()) }
    }
    pub fn get_task_secrets(
        &self,
        _task_id: String,
    ) -> impl Future<Output = anyhow::Result<HashMap<String, ManagedSecretValue>>> + use<RequestScope>
    {
        async {
            Err(anyhow::anyhow!(
                "managed secrets are unavailable in this product profile"
            ))
        }
    }
    pub fn issue_task_identity_token(
        &self,
        _request_scope: Option<RequestScope>,
        _options: IdentityTokenOptions,
    ) -> impl Future<Output = anyhow::Result<TaskIdentityToken>> + use<RequestScope> {
        async {
            Err(anyhow::anyhow!(
                "managed secrets are unavailable in this product profile"
            ))
        }
    }
    pub fn issue_gcp_workload_identity_federation_token(
        &self,
        _audience: String,
        _token_type: String,
        _requested_duration: Duration,
    ) -> impl Future<
        Output = Result<GcpWorkloadIdentityFederationToken, GcpWorkloadIdentityFederationError>,
    > + use<RequestScope> {
        async {
            Err(GcpWorkloadIdentityFederationError::new(
                "managed secrets are unavailable in this product profile",
            ))
        }
    }
}
impl<RequestScope: Send + Sync + 'static> Entity for ManagedSecretManager<RequestScope> {
    type Event = ();
}
impl<RequestScope: Send + Sync + 'static> SingletonEntity for ManagedSecretManager<RequestScope> {}
