//! SSH identity and server management commands.

use crate::error::CliError;
use crate::input;
use crate::session;
use crate::storage;
use std::fs;
use std::io::Write;
use std::process::Command;
use vx_core::ssh;

/// Entry point for SSH command dispatch.
/// Handles `vx ssh init`, `vx ssh connect`, `vx ssh remove`, `vx ssh list`, and `vx ssh <server>`.
pub fn execute(target: Option<String>, args: Vec<String>) -> Result<(), CliError> {
    match target.as_deref() {
        Some("init") => {
            if args.is_empty() {
                return Err(CliError::Generic("Usage: vx ssh init <name>".to_string()));
            }
            init(&args[0])
        }
        Some("add") => {
            if args.is_empty() {
                return Err(CliError::Generic("Usage: vx ssh add <name> <user@host>".to_string()));
            }
            if args.len() < 2 {
                // Interactive setup
                setup_server(&args[0])
            } else {
                // Direct setup: vx ssh add <name> <user@host>
                add_server(&args[0], &args[1])
            }
        }
        Some("remove") => {
            if args.is_empty() {
                return Err(CliError::Generic("Usage: vx ssh remove <name>".to_string()));
            }
            remove(&args[0])
        }
        Some("list") => {
            list_ssh()
        }
        Some("connect") => {
            if args.is_empty() {
                 return Err(CliError::Generic("Usage: vx ssh connect <identity_or_server> [target] [args...]".to_string()));
            }
            let identity_or_server = &args[0];
            let target = args.get(1).map(|s| s.as_str());
            let extra_args = if args.len() > 2 {
                args[2..].to_vec()
            } else {
                Vec::new()
            };
            connect_dispatch(identity_or_server, target, &extra_args)
        }
        Some(other) => {
            // Treat as server name or identity
            // Check if it looks like an identity+target or just server
            // Since we flattened args, `other` is the first arg.
            // If `other` is a server name, we connect to it.
            // If `other` is identity, we expect next arg to be user@host.
            
            // We use connect_dispatch logic but need to be careful with args
            // args here are the *rest* of the arguments.
            
            // Case 1: vx ssh <server> [cmd...]
            // Case 2: vx ssh <identity> <user@host> [cmd...]
            
            // To distinguish, we load vault and check if `other` is a server.
            // But loading vault is expensive? Not really.
            
            // However, `connect_dispatch` already does the check!
            // But `connect_dispatch` signature expects separate args.
            
            // If `other` is server: target is None, extra_args are `args`.
            // If `other` is identity: target is `args[0]`, extra_args are `args[1..]`.
            
            // Let's modify logic to try server first.
            
            // Load vault to check if it's a server
             let (vault, _) = storage::load_vault_with_key_auto()?;
             
             if vault.has_ssh_server(other) {
                 // It's a server
                 let extra_args = args; // all remaining args are for the command
                 // We call the internal connect logic directly to avoid re-loading vault?
                 // Or just delegate.
                 // Since we already loaded vault, we should pass it or re-load.
                 // storage::load_vault... handles caching so re-load is cheap.
                 connect_dispatch(other, None, &extra_args)
             } else {
                 // Not a server, assume identity
                 if args.is_empty() {
                      // If it's not a server and no target provided, maybe they meant to connect to a server that doesn't exist?
                      // or they provided identity but forgot target.
                      // Let's assume they meant a server and let connect_dispatch fail with "Server not found" or try setup.
                      connect_dispatch(other, None, &args)
                 } else {
                     let tgt = &args[0];
                     let extra_args = args[1..].to_vec();
                     connect_dispatch(other, Some(tgt), &extra_args)
                 }
             }
        }
        None => {
            // No arguments provided - show help
            println!("SSH identity and server management\n");
            println!("Usage:");
            println!("  vx ssh init <name>            Create new SSH identity");
            println!("  vx ssh add <name> <user@host> Configure server connection");
            println!("  vx ssh <server>               Connect to configured server");
            println!("  vx ssh list                   List all identities and servers");
            println!("  vx ssh remove <name>          Remove server/identity");
            Ok(())
        }
    }
}


/// Executes the ssh init command.
pub fn init(name: &str) -> Result<(), CliError> {
    // Load or create vault
    let (mut vault, encryption_key, password_bytes) = if storage::vault_exists()? {
        // Load existing vault with cache check
        if let Some(cached) = session::get_cached_password()? {
            match storage::load_vault_with_key(&cached) {
                Ok((v, k)) => (v, k, cached),
                Err(_) => {
                    let _ = session::clear_cached_password();
                    let p = input::read_password("Enter master password: ")?;
                    let (v, k) = storage::load_vault_with_key(p.as_bytes())?;
                    (v, k, p.into_bytes())
                }
            }
        } else {
             let p = input::read_password("Enter master password: ")?;
             let (v, k) = storage::load_vault_with_key(p.as_bytes())?;
             (v, k, p.into_bytes())
        }
    } else {
        println!("Creating new vault...");
        let password = input::read_new_password()?;
        let (vault, key) = storage::create_vault(password.as_bytes())?;
        (vault, key, password.into_bytes())
    };

    // Generate keypair
    let (public_key, private_key) = ssh::generate_keypair().map_err(|e| {
        CliError::SshError(format!("Failed to generate keypair: {}", e))
    })?;

    // Store identity
    vault.add_ssh_identity(name, public_key.clone(), &private_key, &encryption_key)?;

    // Save vault
    storage::save_vault(&vault, &password_bytes)?;

    // Display public key and setup commands
    println!("\n✓ SSH identity '{}' created successfully.\n", name);
    println!("Public key:");
    println!("{}\n", public_key);
    println!("Setup commands for remote server:");
    println!("{}", ssh::generate_setup_commands(&public_key));

    Ok(())
}

/// Dispatches SSH connect based on whether argument is identity or server.
pub fn connect_dispatch(
    identity_or_server: &str,
    target: Option<&str>,
    extra_args: &[String],
) -> Result<(), CliError> {
    // Load vault to check what we're dealing with
    let (vault, encryption_key) = storage::load_vault_with_key_auto()?;

    // Check if it's a configured server
    if vault.has_ssh_server(identity_or_server) {
        // It's a server name - use server shorthand
        connect_server(&vault, &encryption_key, identity_or_server, extra_args)
    } else if let Some(tgt) = target {
        // It's identity + target - use original connect logic
        connect_with_identity(&vault, &encryption_key, identity_or_server, tgt, extra_args)
    } else {
        // Check if it's "connect <servername>" for setup
        // This is the new interactive setup command
        setup_server(identity_or_server)
    }
}

/// Interactive setup for SSH server configuration.
fn setup_server(servername: &str) -> Result<(), CliError> {
    println!("Setting up SSH server configuration: {}", servername);

    // Load vault
    let (mut vault, _encryption_key, password_bytes) = if let Some(cached) = session::get_cached_password()? {
        match storage::load_vault_with_key(&cached) {
            Ok((v, k)) => (v, k, cached),
            Err(_) => {
                let _ = session::clear_cached_password();
                let p = input::read_password("Enter master password: ")?;
                let (v, k) = storage::load_vault_with_key(p.as_bytes())?;
                (v, k, p.into_bytes())
            }
        }
    } else {
         let p = input::read_password("Enter master password: ")?;
         let (v, k) = storage::load_vault_with_key(p.as_bytes())?;
         (v, k, p.into_bytes())
    };

    // Check if server already exists
    if vault.has_ssh_server(servername) {
        if !input::confirm(&format!(
            "Server '{}' already configured. Overwrite?",
            servername
        ))? {
            println!("Cancelled.");
            return Ok(());
        }
    }

    // Check if identity with same name exists
    if !vault.ssh_identities.contains_key(servername) {
        return Err(CliError::SshError(format!(
            "SSH identity '{}' not found. Create it first with: vx ssh init {}",
            servername, servername
        )));
    }

    // Prompt for configuration
    let username = input::read_input("Remote username: ")?;
    if username.is_empty() {
        return Err(CliError::Generic("Username cannot be empty".to_string()));
    }

    let ip_address = input::read_input("Remote IP address: ")?;
    if ip_address.is_empty() {
        return Err(CliError::Generic("IP address cannot be empty".to_string()));
    }

    // Basic IP validation
    validate_ip_or_hostname(&ip_address)?;

    // Save configuration
    vault.add_ssh_server(
        servername,
        username.clone(),
        ip_address.clone(),
        servername.to_string(), // Identity has same name as server
    )?;

    storage::save_vault(&vault, &password_bytes)?;

    println!(
        "\n✓ Server '{}' configured successfully!",
        servername
    );
    println!("  Username: {}", username);
    println!("  IP: {}", ip_address);
    println!("  Identity: {}", servername);
    println!("\nConnect with: vx ssh {}", servername);

    Ok(())
}

/// Connects using a configured server shorthand.
fn connect_server(
    vault: &vx_core::Vault,
    encryption_key: &[u8; 32],
    servername: &str,
    command_args: &[String],
) -> Result<(), CliError> {
    // Get server config
    let server = vault
        .get_ssh_server(servername)
        .map_err(|_| CliError::SshError(format!("Server '{}' not found", servername)))?;

    // Get SSH identity
    let (_public_key, private_key_bytes) = 
        vault.get_ssh_identity(&server.identity_name, encryption_key)?;

    // Build target string
    let target = format!("{}@{}", server.username, server.ip_address);

    // Use existing connection logic
    execute_ssh_connection(&private_key_bytes, &target, &server.identity_name, command_args)
}

/// Connects using identity and target (original behavior).
fn connect_with_identity(
    vault: &vx_core::Vault,
    encryption_key: &[u8; 32],
    identity: &str,
    target: &str,
    extra_args: &[String],
) -> Result<(), CliError> {
    // Get SSH identity
    let (_public_key, private_key_bytes) = vault.get_ssh_identity(identity, encryption_key)?;

    execute_ssh_connection(&private_key_bytes, target, identity, extra_args)
}

/// Common SSH connection execution logic.
///
/// # Security
/// - Decrypts private key in memory
/// - Writes to temp file with 0600 permissions
/// - Deletes temp file after SSH session
fn execute_ssh_connection(
    private_key_bytes: &[u8],
    target: &str,
    identity_name: &str,
    extra_args: &[String],
) -> Result<(), CliError> {
    // Reconstruct signing key and format private key
    let signing_key = ssh::reconstruct_signing_key(private_key_bytes)
        .map_err(|e| CliError::SshError(format!("Invalid key format: {}", e)))?;

    let public_key_bytes = signing_key.verifying_key();
    let private_key_pem = ssh::format_private_key(private_key_bytes, public_key_bytes.as_bytes())
        .map_err(|e| CliError::SshError(format!("Failed to format private key: {}", e)))?;

    // Create temp file for private key
    let temp_dir = tempfile::tempdir()?;
    let key_path = temp_dir.path().join("id_temp");

    // Write private key with restricted permissions
    {
        let mut file = fs::File::create(&key_path)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = fs::Permissions::from_mode(0o600);
            file.set_permissions(permissions)?;
        }

        file.write_all(private_key_pem.as_bytes())?;
        file.sync_all()?;
    }

    // Build SSH command
    let mut cmd = Command::new("ssh");
    cmd.arg("-i").arg(&key_path);
    cmd.arg(target);

    // Add extra arguments
    for arg in extra_args {
        cmd.arg(arg);
    }

    let message = if extra_args.is_empty() {
        format!(
            "Connecting to {} using identity '{}'...\n",
            target, identity_name
        )
    } else {
        format!(
            "Executing command on {} using identity '{}'...\n",
            target, identity_name
        )
    };
    println!("{}", message);

    // Execute SSH with inherited stdio for interactive shell and command output
    let status = cmd
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .map_err(|e| CliError::SshError(format!("Failed to execute ssh: {}", e)))?;

    if !status.success() {
        return Err(CliError::SshError(format!(
            "SSH exited with status: {}",
            status.code().unwrap_or(-1)
        )));
    }

    Ok(())
}

/// Removes an SSH server/identity from the vault.
fn remove(name: &str) -> Result<(), CliError> {
    // Load vault
    let (mut vault, _encryption_key, password_bytes) = if let Some(cached) = session::get_cached_password()? {
        match storage::load_vault_with_key(&cached) {
            Ok((v, k)) => (v, k, cached),
            Err(_) => {
                let _ = session::clear_cached_password();
                let p = input::read_password("Enter master password: ")?;
                let (v, k) = storage::load_vault_with_key(p.as_bytes())?;
                (v, k, p.into_bytes())
            }
        }
    } else {
        let p = input::read_password("Enter master password: ")?;
        let (v, k) = storage::load_vault_with_key(p.as_bytes())?;
        (v, k, p.into_bytes())
    };

    // Check what exists
    let has_server = vault.ssh_servers.contains_key(name);
    let has_identity = vault.ssh_identities.contains_key(name);

    if !has_server && !has_identity {
        return Err(CliError::SshError(format!(
            "No SSH server or identity found with name '{}'",
            name
        )));
    }

    // Confirm removal
    let msg = if has_server && has_identity {
        format!("Remove SSH server AND identity '{}'?", name)
    } else if has_server {
        format!("Remove SSH server '{}'?", name)
    } else {
        format!("Remove SSH identity '{}'? (This will also remove any servers using it)", name)
    };

    if !input::confirm(&msg)? {
        println!("Cancelled.");
        return Ok(());
    }

    // Remove server first (if exists)
    if has_server {
        vault.remove_ssh_server(name)?;
        println!("SSH server '{}' removed.", name);
    }

    // Remove identity (if exists)
    if has_identity {
        vault.remove_ssh_identity(name)?;
        println!("SSH identity '{}' removed.", name);
    }

    // Save vault
    storage::save_vault(&vault, &password_bytes)?;

    Ok(())
}

/// Lists all SSH identities and servers.
fn list_ssh() -> Result<(), CliError> {
    let (vault, _) = storage::load_vault_with_key_auto()?;

    if vault.ssh_identities.is_empty() && vault.ssh_servers.is_empty() {
        println!("No SSH identities or servers configured.");
        println!("\nCreate one with: vx ssh init <name>");
        return Ok(());
    }

    if !vault.ssh_servers.is_empty() {
        println!("SSH Servers:");
        for (name, server) in &vault.ssh_servers {
            println!("  {} → {}@{}", name, server.username, server.ip_address);
        }
        println!();
    }

    // Show standalone identities (not linked to servers)
    let standalone: Vec<_> = vault
        .ssh_identities
        .keys()
        .filter(|name| !vault.ssh_servers.contains_key(*name))
        .collect();

    if !standalone.is_empty() {
        println!("SSH Identities (not configured as servers):");
        for name in standalone {
            println!("  {} (configure with: vx ssh add {})", name, name);
        }
    }

    Ok(())
}

/// Adds a server configuration directly (non-interactive).
fn add_server(name: &str, user_host: &str) -> Result<(), CliError> {
    // Parse user@host
    let parts: Vec<&str> = user_host.splitn(2, '@').collect();
    if parts.len() != 2 {
        return Err(CliError::Generic(
            "Invalid format. Use: vx ssh add <name> <user@host>".to_string(),
        ));
    }
    let username = parts[0];
    let ip_address = parts[1];

    if username.is_empty() || ip_address.is_empty() {
        return Err(CliError::Generic("Username and host cannot be empty".to_string()));
    }

    validate_ip_or_hostname(ip_address)?;

    // Load vault
    let (mut vault, _encryption_key, password_bytes) = if let Some(cached) = session::get_cached_password()? {
        match storage::load_vault_with_key(&cached) {
            Ok((v, k)) => (v, k, cached),
            Err(_) => {
                let _ = session::clear_cached_password();
                let p = input::read_password("Enter master password: ")?;
                let (v, k) = storage::load_vault_with_key(p.as_bytes())?;
                (v, k, p.into_bytes())
            }
        }
    } else {
        let p = input::read_password("Enter master password: ")?;
        let (v, k) = storage::load_vault_with_key(p.as_bytes())?;
        (v, k, p.into_bytes())
    };

    // Check if identity exists
    if !vault.ssh_identities.contains_key(name) {
        return Err(CliError::SshError(format!(
            "SSH identity '{}' not found. Create it first with: vx ssh init {}",
            name, name
        )));
    }

    // Check if server already exists
    if vault.has_ssh_server(name) {
        if !input::confirm(&format!("Server '{}' already configured. Overwrite?", name))? {
            println!("Cancelled.");
            return Ok(());
        }
    }

    // Save configuration
    vault.add_ssh_server(name, username.to_string(), ip_address.to_string(), name.to_string())?;
    storage::save_vault(&vault, &password_bytes)?;

    println!("\n✓ Server '{}' configured!", name);
    println!("  Username: {}", username);
    println!("  IP: {}", ip_address);
    println!("\nConnect with: vx ssh {}", name);

    Ok(())
}

/// Basic IP address or hostname validation.
fn validate_ip_or_hostname(addr: &str) -> Result<(), CliError> {
    if addr.is_empty() {
        return Err(CliError::Generic("Address cannot be empty".to_string()));
    }

    // Allow hostnames, IPv4, and IPv6
    // Basic validation - just check for reasonable characters
    let valid = addr.chars().all(|c| {
        c.is_alphanumeric() || c == '.' || c == ':' || c == '-' || c == '_'
    });

    if !valid {
        return Err(CliError::Generic(format!(
            "Invalid IP address or hostname: {}",
            addr
        )));
    }

    Ok(())
}