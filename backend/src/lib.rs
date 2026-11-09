//! Project-local legacy evidence inspection. The reader has no scheduling API.

pub mod adapters;
pub mod codec;
pub mod context_access;
pub mod db_writer;
pub mod context_policy;
pub mod delta;
pub mod execution;
pub mod fake;
pub mod findings;
pub mod herdr;
pub mod legacy;
pub mod local_context;
pub mod native_policy;
pub mod native_contract;
pub mod conversation_store;
pub mod mock_coordinator;
pub mod static_files;
pub mod operation;
pub mod output;
pub mod output_capture;
pub mod view_registry;
pub mod packet;
pub mod runner;
pub mod session_store;
pub mod session_service;
pub mod systemd_host;
pub mod host_attempt;
pub mod workflow_policy;
pub mod workflow_store;

pub mod claude_chat;

pub mod scope_contract;
pub mod task_scope;

pub mod continuous;
pub mod continuous_adapter;
pub mod development;
pub mod development_context;
pub mod development_recovery;
pub mod review_findings;
pub mod request_history;
pub mod project_layout;
mod worker_activity;

pub mod provider_selection;
pub mod model_runtime;
pub mod coordination;

pub mod request_routing;

pub mod project_workflow;

pub mod project_knowledge;

pub mod candidate;

pub mod project_resources;

pub mod project_publication;
