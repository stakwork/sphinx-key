# steps to integrate LSS

### initialization

##### broker

- check that there is an LSS url to use
- LssClient::get_server_pubkey
- send server pubkey to signer

##### signer

- let client_id = keys_manager.get_persistence_pubkey()
- let auth_token = keys_manager.get_persistence_auth_token(&server_pubkey)
- let shared_secret = keys_manager.get_persistence_shared_secret(&server_pubkey)
- create a ExternalPersistHelper locally and init `state`
- helper.new_nonce
- send the client_id, auth_token, and nonce back to the broker

##### broker

- create Auth
- LssClient::new
- get ALL muts from cloud
- let (muts, server_hmac) = client.get("".to_string(), &nonce)
- send the muts and server_hmac to signer

##### signer

- check the server hmac
- insert the muts into local state
- let handler_builder = handler_builder.lss_state(...);
- (what is the above line do it muts are already inserted???)
- let (handler, muts) = handler_builder.build();
- helper.client_hmac
- send the muts and client_hmac back to broker

##### broker

- store the muts using the LssClient (client.put(muts, &client_hmac))
- send server_hmac back to signer???
- init the Unix Fd connection finally, so the hsmd_init message comes

##### signer

- need to verify server hmac here???

### VLS

##### signer

- let (reply, muts) = handler.handle(msg)
- let client_hmac = helper.client_hmac(&muts);
- send muts and hmac to broker

##### broker

- client.put(muts, &client_hmac).await?
- server hmac sent back to signer

##### signer

- verify server hmac
- finally, send the VLS reply back to broker

##### broker

- forward the VLS reply back to CLN
