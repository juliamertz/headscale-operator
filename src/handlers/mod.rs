pub mod aclpolicy;
pub mod headscale;
pub mod preauth_key;

pub(super) use std::net::SocketAddr;
pub(super) use std::sync::Arc;

pub(super) use anyhow::Context as _;
pub(super) use k8s_openapi_ext::appsv1::*;
pub(super) use k8s_openapi_ext::corev1::*;
pub(super) use k8s_openapi_ext::*;
pub(super) use kube::{Api, Client, Resource, ResourceExt};
pub(super) use kubus::{ApiExt, Context, kubus};
pub(super) use serde::{Deserialize, Serialize};

pub(super) use crate::crds::{aclpolicy::*, headscale::*, preauth_key::*};
pub(super) use crate::ext::ExecuteExt;
pub(super) use crate::{Error, State};
