#!/bin/bash

set -e

function define_vars {
	RELEASE_URL=https://github.com/cluvio/agent/releases/latest
	CONFIG_FILE=agent.toml

	case "$1" in
		"user")
			EXECUTABLE="$HOME/.cluvio/bin/cluvio-agent"
			CONFIG_DIR="$HOME/.cluvio/etc"
			SYSTEMD_UNIT="$HOME/.config/systemd/user/cluvio-agent.service"
			;;
		"system")
			EXECUTABLE=/usr/local/bin/cluvio-agent
			CONFIG_DIR=/usr/local/etc/cluvio
			SYSTEMD_UNIT=/usr/local/lib/systemd/system/cluvio-agent.service
			;;
		*)
			echo "unknown installation mode $1"
			exit 1
	esac
}

function usage {
    cat <<- EOF
		Usage: $(basename $0) [-l <location>] [-v <version>]
		where
		    -l <location> defaults to "eu"
		    -v <version>  defaults to the latest one available
	EOF
}

function install {
	local system="$1"
    local os="$2"
    local cpu="$3"
    local location="$4"
    local version="$5"
    local path="$6"

    if [ -f "$EXECUTABLE" ]; then
        echo -n "$EXECUTABLE already exists. Would you like to upgrade? [y/N]: "
        read answer
        if [ "${answer,,}" = "y" ]; then
            upgrade $os $arch $location $version $path
        fi
        return
    fi

	if [ -f "$CONFIG_DIR/$CONFIG_FILE" ]; then
		echo "$CONFIG_DIR/$CONFIG_FILE already exists, creating backup ..."
		mv -v --backup=numbered "$CONFIG_DIR/$CONFIG_FILE" "$CONFIG_DIR/$CONFIG_FILE.bak"
	fi

    if [ -z $version ]; then
        latest_version
        version=$RETURN
    fi

    archive_name $os $cpu $version $location
    local archive="$RETURN"
    download $archive $path
    sha256sum --quiet -c CHECKSUMS

    echo "Extracting $archive ..."
	mkdir -p "$(dirname $EXECUTABLE)"
	mkdir -p "$CONFIG_DIR"
	tar -xJ -C $(dirname "$EXECUTABLE") -f "$archive" cluvio-agent

    $EXECUTABLE --setup "$CONFIG_DIR/$CONFIG_FILE"
	chmod 0600 "$CONFIG_DIR/$CONFIG_FILE"

    local key=$("$EXECUTABLE" --show-agent-key --config "$CONFIG_DIR/$CONFIG_FILE")

    cat <<- EOF
		Installation complete.
		The agent needs to be registered at https://www.cluvio.com with the following key:

		    $key

	EOF

    if [ "linux" = $os ]; then
        linux_post_install $system
    fi

	echo "Done."
}

function upgrade {
    local os="$1"
    local cpu="$2"
    local location="$3"
    local version="$4"
    local path="$5"

    if [ -z $version ]; then
        latest_version
        version=$RETURN
    fi

    archive_name $os $cpu $version $location
    local archive="$RETURN"
    download $archive $path
    sha256sum --quiet -c CHECKSUMS

    echo "Extracting $archive ..."
	tar -xJ -C $(dirname "$EXECUTABLE") -f "$archive" cluvio-agent

    echo "Upgrade complete."
}

function latest_version {
    local url=$(curl -s -o /dev/null -w "%{redirect_url}" $RELEASE_URL)
    local version=${url##https://*/v}
    if [ -z "$version" ]; then
        echo "Could not get the latest version from $RELEASE_URL."
        exit 1
    fi
    RETURN=$version
}

function download {
    local archive="$1"
    local path="$2"

    if [ -z $path ]; then
        echo "Downloading $archive ..."
        curl --proto '=https' --tlsv1.2 -sSf -L "$RELEASE_URL/$archive" -o "$archive"
        curl --proto '=https' --tlsv1.2 -sSf -L "$RELEASE_URL/CHECKSUMS" -o "CHECKSUMS"
    else
        curl -sSf "file://$path/$archive" -o "$archive"
        curl -sSf "file://$path/CHECKSUMS" -o "CHECKSUMS"
    fi
}

function create_systemd_unit {
    mkdir -p $(dirname "$SYSTEMD_UNIT")
    cat > "$SYSTEMD_UNIT" <<- EOF
		[Unit]
		Description=Cluvio's connection agent
		After=network.target

		[Service]
		Type=simple
		ExecStart=${1}
		Restart=on-abort
		RestartSec=30

		[Install]
		WantedBy=${2}
	EOF
}

function linux_post_install {
	case $1 in
		"user")
			sysctl="systemctl --user"
			target="default.target"
			;;
		"system")
			sysctl="systemctl"
			target="multi-user.target"
			;;
	esac

    echo -en "Would you like to setup the agent for use with systemd? [y/N]: "
    read answer
    if [ "${answer,,}" != "y" ]; then
        cat <<- EOF
			Once the agent has been registered with Cluvio it can be started with:

			    $EXECUTABLE -c $CONFIG_DIR/$CONFIG_FILE

		EOF
		return
	fi

	create_systemd_unit "$EXECUTABLE -c $CONFIG_DIR/$CONFIG_FILE" "$target"

	echo -n "Unit file cluvio-agent.service created. Would you like enable the service? [y/N]: "
	read answer
    if [ "${answer,,}" != "y" ]; then
		cat <<- EOF
			Once the agent has been registered with Cluvio the service can be started with:

			    $sysctl enable cluvio-agent.service
			    $sysctl start cluvio-agent.service

		EOF
		return
	fi

	case $1 in
		"user")
			systemctl --user enable cluvio-agent.service
			;;
		"system")
			systemctl enable cluvio-agent.service
			;;
	esac

	cat <<- EOF
		Once the agent has been registered with Cluvio the service can be started with:

		    $sysctl start cluvio-agent.service

	EOF
}

function operating_system {
    local os="$(uname -s)"

    case "$os" in
        Linux)
            os=linux
            ;;
        Darwin)
            os=darwin
            ;;
        *)
            echo "operating system $os is not supported"
            exit 1
            ;;
    esac

    RETURN=$os
}

function cpu_arch {
    local cpu="$(uname -m)"

    case "$cpu" in
        x86_64 | x86-64 | x64 | amd64)
            cpu=x86_64
            ;;
        aarch64 | arm64)
            cpu=aarch64
            ;;
        *)
            echo "cpu type $cpu is not supported"
            exit 1
            ;;
    esac

    RETURN=$cpu
}

function archive_name {
    local os="$1"
    local cpu="$2"
    local version="$3"
    local location="$4"

    case "$os" in
        "linux")
            RETURN="agent-$location-$version-$cpu-unknown-linux-musl.tar.xz"
            ;;
        "darwin")
            RETURN="agent-$location-$version-$cpu-apple-darwin.tar.gz"
            ;;
        *)
            echo "unknown operating system $os"
            exit 1
            ;;
    esac
}

while getopts ":sl:v:p:" o; do
    case "$o" in
        l)
            location=$OPTARG
            ;;
        v)
            version=$OPTARG
            ;;
        p)
            path=$OPTARG
            ;;
        *)
            usage
            exit 1
            ;;
    esac
done

shift $((OPTIND-1))

operating_system
os=$RETURN

cpu_arch
cpu=$RETURN

wd=$(mktemp -d -t cluvio-XXXXX)

echo -n "Install the Cluvio agent for the current user [u] or system-wide [s]? [U/s]: "
read answer

case "${answer,,}" in
    "s")
        if [ "0" != "$(id -u)" ]; then
            echo "A system-wide installation requires root permissions."
            exit 1
        fi
        mode="system"
        ;;
    "u" | "")
        mode="user"
        ;;
    *)
        echo "Invalid input ${answer}."
        exit 1
        ;;
esac

define_vars $mode
(cd $wd && install $mode $os $cpu "${location:-eu}" $version $path)

