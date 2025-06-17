import toml
import requests

import importlib.metadata


def check_version():
    try:
        local_version = importlib.metadata.version("bond")
    except importlib.metadata.PackageNotFoundError:
        print("Error: 'bond' package not found. Please ensure it's installed.")
        return

    remote_url = (
        "https://raw.githubusercontent.com/luka598/bond/refs/heads/main/pyproject.toml"
    )
    try:
        response = requests.get(remote_url)
        response.raise_for_status()
        remote_config = toml.loads(response.text)
        remote_version = remote_config["project"]["version"]
    except requests.exceptions.RequestException as e:
        print(f"Error fetching remote pyproject.toml: {e}")
        return
    except Exception as e:
        print(f"Error parsing remote pyproject.toml: {e}")
        return

    if local_version != remote_version:
        print(
            f"Your program is out of date.\nLocal version: {local_version} | Remote version {remote_version}."
        )
