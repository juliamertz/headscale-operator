use k8s_openapi::api::core::v1::ServiceAccount;
use k8s_openapi::api::rbac::v1::{Role, RoleBinding, RoleRef, Subject};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{ObjectMeta, OwnerReference};
use kube::api::{Api, DeleteParams, Patch, PatchParams, PostParams};
use kube::{Client, Error as KubeError, Resource, ResourceExt};
use serde::de::DeserializeOwned;
use std::collections::BTreeMap;
use std::fmt::Debug;

use crate::Error;

async fn create_or_update<K>(api: &Api<K>, name: &str, resource: &K) -> Result<(), Error>
where
    K: Resource<DynamicType = ()>
        + Clone
        + Send
        + Sync
        + Debug
        + DeserializeOwned
        + serde::Serialize,
{
    match api.get(name).await {
        Ok(_) => {
            api.patch(name, &PatchParams::default(), &Patch::Merge(resource))
                .await?;
        }
        Err(KubeError::Api(kube::error::ErrorResponse { code: 404, .. })) => {
            api.create(&PostParams::default(), resource).await?;
        }
        Err(e) => return Err(e.into()),
    }
    Ok(())
}

async fn delete_if_exists<K>(api: &Api<K>, name: &str) -> Result<(), Error>
where
    K: Resource<DynamicType = ()> + Clone + Send + Sync + Debug + DeserializeOwned,
{
    match api.delete(name, &DeleteParams::default()).await {
        Ok(_) => Ok(()),
        Err(KubeError::Api(kube::error::ErrorResponse { code: 404, .. })) => Ok(()),
        Err(e) => Err(e.into()),
    }
}

pub trait Rbac {
    fn service_account(&self) -> &ServiceAccount;
    fn role(&self) -> &Role;
    fn role_binding(&self) -> &RoleBinding;

    async fn apply(&self, client: &Client, namespace: &str) -> Result<(), Error> {
        let service_account = self.service_account();
        let role = self.role();
        let role_binding = self.role_binding();

        let service_account_api: Api<ServiceAccount> = Api::namespaced(client.clone(), namespace);
        create_or_update(
            &service_account_api,
            &service_account.name_any(),
            service_account,
        )
        .await?;

        let role_api: Api<Role> = Api::namespaced(client.clone(), namespace);
        create_or_update(&role_api, &role.name_any(), role).await?;

        let role_binding_api: Api<RoleBinding> = Api::namespaced(client.clone(), namespace);
        create_or_update(&role_binding_api, &role_binding.name_any(), role_binding).await?;

        Ok(())
    }

    async fn delete(&self, client: &Client, namespace: &str) -> Result<(), Error> {
        let service_account = self.service_account();
        let role = self.role();
        let role_binding = self.role_binding();

        let role_binding_api: Api<RoleBinding> = Api::namespaced(client.clone(), namespace);
        delete_if_exists(&role_binding_api, &role_binding.name_any()).await?;

        let role_api: Api<Role> = Api::namespaced(client.clone(), namespace);
        delete_if_exists(&role_api, &role.name_any()).await?;

        let service_account_api: Api<ServiceAccount> = Api::namespaced(client.clone(), namespace);
        delete_if_exists(&service_account_api, &service_account.name_any()).await?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ConfigManagerRbac {
    pub service_account: ServiceAccount,
    pub role: Role,
    pub role_binding: RoleBinding,
}

impl ConfigManagerRbac {
    pub fn new(
        name: &str,
        namespace: &str,
        acl_configmap_name: &str,
        owner_ref: OwnerReference,
        labels: impl Iterator<Item = (&'static str, String)>,
    ) -> Self {
        let base_name = format!("{name}-config-manager");
        let mut label_map = BTreeMap::new();
        for (k, v) in labels {
            label_map.insert(k.to_string(), v);
        }

        let service_account = ServiceAccount {
            metadata: ObjectMeta {
                name: Some(base_name.clone()),
                namespace: Some(namespace.to_string()),
                labels: Some(label_map.clone()),
                owner_references: Some(vec![owner_ref.clone()]),
                ..Default::default()
            },
            ..Default::default()
        };

        let role = Role {
            metadata: ObjectMeta {
                name: Some(base_name.clone()),
                namespace: Some(namespace.to_string()),
                labels: Some(label_map.clone()),
                owner_references: Some(vec![owner_ref.clone()]),
                ..Default::default()
            },
            rules: Some(vec![k8s_openapi::api::rbac::v1::PolicyRule {
                api_groups: Some(vec!["".to_string()]),
                resources: Some(vec!["configmaps".to_string()]),
                resource_names: Some(vec![acl_configmap_name.to_string()]),
                verbs: vec!["get".to_string(), "list".to_string(), "watch".to_string()],
                ..Default::default()
            }]),
        };

        let role_binding = RoleBinding {
            metadata: ObjectMeta {
                name: Some(base_name.clone()),
                namespace: Some(namespace.to_string()),
                labels: Some(label_map),
                owner_references: Some(vec![owner_ref]),
                ..Default::default()
            },
            role_ref: RoleRef {
                api_group: "rbac.authorization.k8s.io".to_string(),
                kind: "Role".to_string(),
                name: base_name.clone(),
            },
            subjects: Some(vec![Subject {
                kind: "ServiceAccount".to_string(),
                name: base_name,
                namespace: Some(namespace.to_string()),
                ..Default::default()
            }]),
        };

        Self {
            service_account,
            role,
            role_binding,
        }
    }
}

impl Rbac for ConfigManagerRbac {
    fn service_account(&self) -> &ServiceAccount {
        &self.service_account
    }

    fn role(&self) -> &Role {
        &self.role
    }

    fn role_binding(&self) -> &RoleBinding {
        &self.role_binding
    }
}
