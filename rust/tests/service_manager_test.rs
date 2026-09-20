use mirror_core::service_manager::*;

#[test]
fn test_is_elevated_query_does_not_panic() {
    let elevated = is_elevated();
    println!("Process elevated status: {}", elevated);
}

#[test]
fn test_get_service_status_query() {
    let status = get_service_status();
    println!("Service status info: {:?}", status);
    // Since this runs in standard test runner, verify structure fields
    assert_eq!(status.is_service, false);
}

#[test]
fn test_pipe_request_response_serialization() {
    let req = PipeRequest {
        cmd: "is_elevated".to_string(),
        args: None,
    };
    let json_str = serde_json::to_string(&req).expect("serialize req");
    let req_deser: PipeRequest = serde_json::from_str(&json_str).expect("deserialize req");
    assert_eq!(req_deser.cmd, "is_elevated");

    let resp = PipeResponse {
        status: "ok".to_string(),
        message: Some("Elevated query passed".to_string()),
        elevated: true,
        is_service: true,
    };
    let json_resp = serde_json::to_string(&resp).expect("serialize resp");
    let resp_deser: PipeResponse = serde_json::from_str(&json_resp).expect("deserialize resp");
    assert_eq!(resp_deser.status, "ok");
    assert_eq!(resp_deser.elevated, true);
    assert_eq!(resp_deser.is_service, true);
}

#[tokio::test]
async fn test_pipe_ipc_client_server_exchange() {
    #[cfg(windows)]
    {
        // Start background named pipe server
        let server_handle = tokio::spawn(async {
            let _ = pipe_ipc::run_server(false).await;
        });

        // Give server a moment to bind the pipe
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;

        // Send ping command
        let ping_req = PipeRequest {
            cmd: "ping".to_string(),
            args: None,
        };
        let ping_resp = pipe_ipc::send_command(&ping_req).await;
        assert!(ping_resp.is_ok(), "Ping failed: {:?}", ping_resp.err());
        let ping_val = ping_resp.unwrap();
        assert_eq!(ping_val.status, "ok");

        // Send is_elevated command
        let elev_req = PipeRequest {
            cmd: "is_elevated".to_string(),
            args: None,
        };
        let elev_resp = pipe_ipc::send_command(&elev_req).await;
        assert!(
            elev_resp.is_ok(),
            "is_elevated failed: {:?}",
            elev_resp.err()
        );
        let elev_val = elev_resp.unwrap();
        assert_eq!(elev_val.status, "ok");

        // Send raw JSON via send_pipe_command
        let raw_resp = send_pipe_command(r#"{"cmd":"ping"}"#).await;
        assert!(
            raw_resp.is_ok(),
            "send_pipe_command failed: {:?}",
            raw_resp.err()
        );

        server_handle.abort();
    }
}
