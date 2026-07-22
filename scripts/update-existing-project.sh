#!/bin/sh

set -eu

usage() {
    cat <<'EOF'
Usage: scripts/update-existing-project.sh [OPTIONS] PROJECT [PROJECT ...]

Update existing belay-trace projects with the binary built from this checkout.
Generated assets are always refreshed. Existing AGENTS.md, Codex skill, and
Claude skill integrations are updated only when already active.

Options:
  --belay PATH          Use PATH instead of building target/release/belay
  --update-agents       Add or update the managed AGENTS.md section
  --install-codex       Install or update the repository Codex skill
  --install-claude      Install or update the repository Claude skill
  --reset-state         Rebuild ignored SQLite state from managed Markdown
  --initialize          Allow projects without .belay/config.toml
  -h, --help            Show this help
EOF
}

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
source_root=$(dirname -- "$script_dir")
belay_bin=
update_agents=false
install_codex=false
install_claude=false
reset_state=false
allow_initialize=false

while [ "$#" -gt 0 ]; do
    case "$1" in
        --belay)
            if [ "$#" -lt 2 ]; then
                echo "error: --belay requires a path" >&2
                exit 2
            fi
            belay_bin=$2
            shift 2
            ;;
        --update-agents)
            update_agents=true
            shift
            ;;
        --install-codex)
            install_codex=true
            shift
            ;;
        --install-claude)
            install_claude=true
            shift
            ;;
        --reset-state)
            reset_state=true
            shift
            ;;
        --initialize)
            allow_initialize=true
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        --)
            shift
            break
            ;;
        -*)
            echo "error: unknown option: $1" >&2
            usage >&2
            exit 2
            ;;
        *)
            break
            ;;
    esac
done

if [ "$#" -eq 0 ]; then
    echo "error: provide at least one project path" >&2
    usage >&2
    exit 2
fi

if [ -z "$belay_bin" ]; then
    echo "Building belay from $source_root"
    cargo build --release --locked --manifest-path "$source_root/Cargo.toml"
    belay_bin=$source_root/target/release/belay
fi

case "$belay_bin" in
    /*) ;;
    *) belay_bin="$(CDPATH= cd -- "$(dirname -- "$belay_bin")" && pwd)/$(basename -- "$belay_bin")" ;;
esac

if [ ! -x "$belay_bin" ]; then
    echo "error: belay binary is not executable: $belay_bin" >&2
    exit 2
fi

for requested_project in "$@"; do
    if [ ! -d "$requested_project" ]; then
        echo "error: project directory does not exist: $requested_project" >&2
        exit 2
    fi
    project=$(CDPATH= cd -- "$requested_project" && pwd)

    if [ ! -f "$project/.belay/config.toml" ] && [ "$allow_initialize" != true ]; then
        echo "error: not an initialized belay project: $project" >&2
        echo "hint: pass --initialize to initialize it explicitly" >&2
        exit 2
    fi

    project_update_agents=$update_agents
    project_install_codex=$install_codex
    project_install_claude=$install_claude

    if [ -f "$project/AGENTS.md" ] && grep -q '<!-- belay-trace:start -->' "$project/AGENTS.md"; then
        project_update_agents=true
    fi
    if [ -f "$project/.agents/skills/belay-trace/SKILL.md" ]; then
        project_install_codex=true
    fi
    if [ -f "$project/.claude/skills/belay-trace/SKILL.md" ]; then
        project_install_claude=true
    fi

    echo "Updating $project"
    (cd "$project" && "$belay_bin" init)
    if [ "$project_update_agents" = true ]; then
        (cd "$project" && "$belay_bin" init --update-agents)
    fi
    if [ "$project_install_codex" = true ]; then
        (cd "$project" && "$belay_bin" init --install-skill codex)
    fi
    if [ "$project_install_claude" = true ]; then
        (cd "$project" && "$belay_bin" init --install-skill claude)
    fi
    if [ "$reset_state" = true ]; then
        (cd "$project" && "$belay_bin" init --reset-state)
    fi
    (cd "$project" && "$belay_bin" doctor)
done
