import json

def get_user_input(prompt, cast_type, validation_func=None):
    """Utility function to get validated user input."""
    while True:
        user_input = input(prompt)
        try:
            value = cast_type(user_input)
            if validation_func and not validation_func(value):
                raise ValueError("Invalid value")
            return value
        except ValueError as e:
            print(f"Invalid input: {e}. Please try again.")

def configure_cpu():
    """Configure CPU restrictions."""
    print("\n--- CPU Configuration ---")
    percent = get_user_input("Enter CPU limit as a percentage (1-100): ", int, lambda x: 1 <= x <= 100)
    return {"Percent": percent}

def configure_memory():
    """Configure memory restrictions."""
    print("\n--- Memory Configuration ---")
    memory = get_user_input("Enter max memory in MB: ", int, lambda x: x > 0)
    return memory

def configure_io():
    """Configure I/O restrictions."""
    print("\n--- I/O Configuration ---")
    read_speed_max = get_user_input("Enter max I/O read speed in MB/s (leave blank for no limit): ", int, lambda x: x >= 0 or x == '')
    write_speed_max = get_user_input("Enter max I/O write speed in MB/s (leave blank for no limit): ", int, lambda x: x >= 0 or x == '')
    riops = get_user_input("Enter max read IOPS (leave blank for no limit): ", int, lambda x: x >= 0 or x == '')
    wiops = get_user_input("Enter max write IOPS (leave blank for no limit): ", int, lambda x: x >= 0 or x == '')
    return {"read_speed_max": read_speed_max if read_speed_max != '' else None, "write_speed_max": write_speed_max if write_speed_max != '' else None, "riops": riops if riops != '' else None, "wiops": wiops if wiops != '' else None}

def configure_path_restriction():
    """Configure path restrictions."""
    print("\n--- Path Restrictions Configuration ---")
    mode_input = get_user_input("Choose path restriction mode (1 for Whitelist, 2 for Blacklist): ", int, lambda x: x in [1, 2])
    mode = "WhiteList" if mode_input == 1 else "BlackList"
    paths = []
    while True:
        path = input("Enter a path (leave blank to finish): ").strip()
        if not path:
            break
        paths.append(path)
    return {"mode": mode, "list": paths}

def main():
    print("Configuring Personas...")
    persona_id = get_user_input("Enter persona ID: ", int, lambda x: x >= 0)
    
    personas_config = {
        "personas_id": persona_id,
        "cpu": configure_cpu(),
        "memory": configure_memory(),
        "io": configure_io(),
        "isolated_fs": get_user_input("Isolate filesystem? (yes/no): ", str, lambda x: x.lower() in ['yes', 'no']) == 'yes',
        "device_access": get_user_input("Allow device access? (yes/no): ", str, lambda x: x.lower() in ['yes', 'no']) == 'yes',
        "path_restriction": configure_path_restriction(),
    }

    with open("config.json", "w") as config_file:
        json.dump(personas_config, config_file, indent=4)
    print("Configuration saved to config.json.")

if __name__ == "__main__":
    main()

