use eve_rs::AsError;
use hex::ToHex;
use tokio::{net::UdpSocket, task};
use trust_dns_client::{
	client::{AsyncClient, ClientHandle},
	rr::{DNSClass, Name, RData, RecordType},
	udp::UdpClientStream,
};
use uuid::Uuid;

use crate::{
	db,
	error,
	models::rbac,
	utils::{
		constants::ResourceOwnerType,
		get_current_time_millis,
		validator,
		Error,
	},
	Database,
};

pub async fn get_metrics(
	connection: &mut <Database as sqlx::Database>::Connection,
	metric_type: &str,
) -> Result<(), Error> {
    let metrics = match metric_type {
        "sign-up" => get_cpu_metrics(connection).await?,
        "join" => get_memory_metrics(connection).await?,
        "create-deployment" => get_disk_metrics(connection).await?,
        "update-deployment-domain" => get_network_metrics(connection).await?,
        "validate-deployment-domain" => get_network_metrics(connection).await?,
        "delete-deployment" => get_process_metrics(connection).await?,
        "create-database" => get_system_metrics(connection).await?,
        "delete-database" => get_system_metrics(connection).await?,
        "create-static-site" => get_system_metrics(connection).await?,
        "update-static-site-domain" => get_system_metrics(connection).await?,
        "validate-static-site-doamin" => get_system_metrics(connection).await?,
        _ => {
            return Err(error::ErrorInternalServerError(
                "Metric type not found".to_string(),
            ))
        }
    };
	Ok(())
}
