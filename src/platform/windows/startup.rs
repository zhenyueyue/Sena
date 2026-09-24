use std::{env, io};

use windows::{
    Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_SZ, RegCloseKey,
        RegCreateKeyW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
    },
    core::w,
};

const VALUE_NAME: windows::core::PCWSTR = w!("Sena");
const RUN_KEY: windows::core::PCWSTR = w!(r"Software\Microsoft\Windows\CurrentVersion\Run");

struct RegistryKey(HKEY);

impl Drop for RegistryKey {
    fn drop(&mut self) {
        unsafe {
            let _ = RegCloseKey(self.0);
        }
    }
}

pub fn startup_enabled() -> bool {
    read_startup_command().is_some()
}

pub fn set_startup_enabled(enabled: bool) -> io::Result<()> {
    if enabled {
        let executable = env::current_exe()?;
        let command = format!(r#""{}""#, executable.display());
        write_startup_command(&command)
    } else {
        delete_startup_command()
    }
}

fn read_startup_command() -> Option<String> {
    let key = open_run_key(KEY_QUERY_VALUE).ok()?;

    let mut value_type = Default::default();
    let mut size = 0u32;
    let status = unsafe {
        RegQueryValueExW(
            key.0,
            VALUE_NAME,
            None,
            Some(&mut value_type),
            None,
            Some(&mut size),
        )
    };
    if status.0 != 0 || value_type != REG_SZ || size < 2 {
        return None;
    }

    let mut bytes = vec![0u8; size as usize];
    let status = unsafe {
        RegQueryValueExW(
            key.0,
            VALUE_NAME,
            None,
            Some(&mut value_type),
            Some(bytes.as_mut_ptr()),
            Some(&mut size),
        )
    };
    if status.0 != 0 || value_type != REG_SZ {
        return None;
    }

    bytes.truncate(size as usize);
    let utf16: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .take_while(|unit| *unit != 0)
        .collect();

    String::from_utf16(&utf16).ok()
}

fn write_startup_command(command: &str) -> io::Result<()> {
    let key = create_run_key()?;
    let mut utf16: Vec<u16> = command.encode_utf16().collect();
    utf16.push(0);

    let bytes: Vec<u8> = utf16.iter().flat_map(|unit| unit.to_le_bytes()).collect();
    let status = unsafe { RegSetValueExW(key.0, VALUE_NAME, None, REG_SZ, Some(&bytes)) };

    win32_status(status.0, "write Sena startup entry")
}

fn delete_startup_command() -> io::Result<()> {
    let Ok(key) = open_run_key(KEY_SET_VALUE) else {
        return Ok(());
    };

    let status = unsafe { RegDeleteValueW(key.0, VALUE_NAME) };
    // ERROR_FILE_NOT_FOUND means it was already disabled.
    if status.0 == 0 || status.0 == 2 {
        Ok(())
    } else {
        win32_status(status.0, "delete Sena startup entry")
    }
}

fn open_run_key(
    access: windows::Win32::System::Registry::REG_SAM_FLAGS,
) -> io::Result<RegistryKey> {
    let mut key = HKEY::default();
    let status = unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, RUN_KEY, None, access, &mut key) };
    win32_status(status.0, "open current-user Run key")?;
    Ok(RegistryKey(key))
}

fn create_run_key() -> io::Result<RegistryKey> {
    let mut key = HKEY::default();
    let status = unsafe { RegCreateKeyW(HKEY_CURRENT_USER, RUN_KEY, &mut key) };
    win32_status(status.0, "create current-user Run key")?;
    Ok(RegistryKey(key))
}

fn win32_status(code: u32, action: &str) -> io::Result<()> {
    if code == 0 {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "{action} failed with Win32 error {code}"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_command_quotes_executable_path_shape() {
        let executable = std::path::Path::new(r"C:\Program Files\Sena\sena.exe");
        let command = format!(r#""{}""#, executable.display());

        assert_eq!(command, r#""C:\Program Files\Sena\sena.exe""#);
    }

    #[test]
    #[ignore = "manual Windows registry integration test; restores the previous Sena Run value"]
    fn startup_registry_round_trip_restores_previous_state() {
        struct RestoreStartup(Option<String>);

        impl Drop for RestoreStartup {
            fn drop(&mut self) {
                match self.0.as_deref() {
                    Some(command) => {
                        let _ = write_startup_command(command);
                    }
                    None => {
                        let _ = delete_startup_command();
                    }
                }
            }
        }

        let original = read_startup_command();
        let _restore = RestoreStartup(original);

        set_startup_enabled(true).expect("startup entry should enable");
        assert!(startup_enabled());
        assert!(
            read_startup_command()
                .as_deref()
                .is_some_and(|command| command.starts_with('"') && command.ends_with('"'))
        );

        set_startup_enabled(false).expect("startup entry should disable");
        assert!(!startup_enabled());
    }
}
