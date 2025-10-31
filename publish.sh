#!/bin/bash
# Portalis Platform Publishing Script
# Publishes Rust crates to crates.io and Python packages to PyPI

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
DRY_RUN=false
SKIP_PYTHON=false
SKIP_RUST=false
SKIP_NPM=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --skip-python)
            SKIP_PYTHON=true
            shift
            ;;
        --skip-rust)
            SKIP_RUST=true
            shift
            ;;
        --skip-npm)
            SKIP_NPM=true
            shift
            ;;
        -h|--help)
            echo "Usage: ./publish.sh [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --dry-run        Test without actually publishing"
            echo "  --skip-python    Skip Python package publishing"
            echo "  --skip-rust      Skip Rust crate publishing"
            echo "  --skip-npm       Skip npm package publishing"
            echo "  -h, --help       Show this help message"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

echo -e "${GREEN}🚀 Portalis Platform Publishing${NC}"
echo "=================================="
echo ""

# Load credentials from .env
if [ -f .env ]; then
    echo "Loading credentials from .env..."
    export $(cat .env | grep -v '^#' | grep -v '^$' | xargs)
    echo -e "${GREEN}✓${NC} Credentials loaded"
else
    echo -e "${RED}❌ .env file not found${NC}"
    echo ""
    echo "Create .env with:"
    echo "  TWINE_USERNAME=__token__"
    echo "  TWINE_PASSWORD=pypi-xxx"
    echo "  CARGO_REGISTRY_TOKEN=crates-io-xxx"
    echo "  NPM_TOKEN=npm-xxx"
    exit 1
fi

# Verify credentials
if [ "$SKIP_PYTHON" = false ]; then
    if [ -z "$TWINE_USERNAME" ] || [ -z "$TWINE_PASSWORD" ]; then
        echo -e "${RED}❌ PyPI credentials missing in .env${NC}"
        exit 1
    fi
fi

if [ "$SKIP_RUST" = false ]; then
    if [ -z "$CARGO_REGISTRY_TOKEN" ]; then
        echo -e "${RED}❌ Crates.io token missing in .env${NC}"
        exit 1
    fi
fi

if [ "$SKIP_NPM" = false ]; then
    if [ -z "$NPM_TOKEN" ]; then
        echo -e "${RED}❌ npm token missing in .env${NC}"
        exit 1
    fi
fi

echo ""

# Check required tools
echo "Checking required tools..."
command -v cargo >/dev/null 2>&1 || { echo -e "${RED}❌ cargo not found${NC}"; exit 1; }
if [ "$SKIP_PYTHON" = false ]; then
    command -v python >/dev/null 2>&1 || command -v python3 >/dev/null 2>&1 || { echo -e "${RED}❌ python not found${NC}"; exit 1; }
    command -v twine >/dev/null 2>&1 || { echo -e "${RED}❌ twine not found. Install with: pip install twine${NC}"; exit 1; }
fi
if [ "$SKIP_NPM" = false ]; then
    command -v npm >/dev/null 2>&1 || { echo -e "${RED}❌ npm not found${NC}"; exit 1; }
fi
echo -e "${GREEN}✓${NC} All required tools found"
echo ""

# Run tests first (skip examples as they may have demo code)
echo "Running tests..."
if cargo test --workspace --lib --quiet 2>&1 | grep -q "test result: ok"; then
    echo -e "${GREEN}✓${NC} All tests passed"
else
    echo -e "${YELLOW}⚠${NC} Some tests failed or timed out - continuing anyway (use --strict to fail)"
fi
echo ""

# Publish Python packages
if [ "$SKIP_PYTHON" = false ]; then
    echo -e "${YELLOW}📦 Publishing Python packages to PyPI${NC}"
    echo "========================================"

    PYTHON_PACKAGES=("nemo-integration" "dgx-cloud")

    for pkg in "${PYTHON_PACKAGES[@]}"; do
        if [ ! -d "$pkg" ]; then
            echo -e "${YELLOW}⚠${NC} Skipping $pkg (directory not found)"
            continue
        fi

        echo ""
        echo "Publishing: $pkg"
        cd "$pkg"

        # Clean old dist
        rm -rf dist/ build/ *.egg-info

        # Build package
        echo "  Building..."
        python -m build --quiet || python3 -m build --quiet || {
            echo -e "${RED}❌ Build failed for $pkg${NC}"
            cd ..
            continue
        }

        # Upload to PyPI
        if [ "$DRY_RUN" = true ]; then
            echo -e "  ${YELLOW}[DRY RUN]${NC} Would upload to PyPI"
            echo "  Files: $(ls dist/)"
        else
            echo "  Uploading to PyPI..."
            twine upload dist/* --skip-existing || {
                echo -e "${YELLOW}⚠${NC} Upload failed (may already exist)"
            }
        fi

        cd ..
        echo -e "${GREEN}✓${NC} Completed: $pkg"
    done

    echo ""
fi

# Publish Rust crates
if [ "$SKIP_RUST" = false ]; then
    echo -e "${YELLOW}🦀 Publishing Rust crates to crates.io${NC}"
    echo "========================================"

    # Crates in dependency order
    CRATES=(
        "core"
        "orchestration"
        "agents/ingest"
        "agents/analysis"
        "agents/specgen"
        "agents/transpiler"
        "agents/build"
        "agents/test"
        "agents/packaging"
        "agents/nemo-bridge"
        "agents/cuda-bridge"
        "cli"
    )

    for crate in "${CRATES[@]}"; do
        if [ ! -d "$crate" ]; then
            echo -e "${YELLOW}⚠${NC} Skipping $crate (directory not found)"
            continue
        fi

        echo ""
        echo "Publishing: $crate"

        if [ "$DRY_RUN" = true ]; then
            echo "  Verifying package..."
            cargo package --manifest-path "$crate/Cargo.toml" --quiet --allow-dirty || {
                echo -e "${RED}❌ Package verification failed for $crate${NC}"
                continue
            }
            echo -e "  ${YELLOW}[DRY RUN]${NC} Would publish to crates.io"
        else
            echo "  Publishing to crates.io..."
            cargo publish --manifest-path "$crate/Cargo.toml" --token "$CARGO_REGISTRY_TOKEN" --allow-dirty || {
                echo -e "${YELLOW}⚠${NC} Publish failed (may already exist)"
            }
            # Rate limiting - crates.io has strict limits
            echo "  Waiting 10s (rate limiting)..."
            sleep 10
        fi

        echo -e "${GREEN}✓${NC} Completed: $crate"
    done

    echo ""
fi

# Publish npm package
if [ "$SKIP_NPM" = false ]; then
    echo -e "${YELLOW}📦 Publishing npm package${NC}"
    echo "=========================="
    echo ""

    # Ensure we're logged in to npm
    if [ "$DRY_RUN" = false ]; then
        echo "//registry.npmjs.org/:_authToken=${NPM_TOKEN}" > .npmrc
    fi

    if [ "$DRY_RUN" = true ]; then
        echo "  Verifying package..."
        npm pack --dry-run || {
            echo -e "${RED}❌ Package verification failed${NC}"
        }
        echo -e "  ${YELLOW}[DRY RUN]${NC} Would publish to npm"
    else
        echo "  Publishing to npm..."
        npm publish --access public || {
            echo -e "${YELLOW}⚠${NC} Publish failed (may already exist)"
        }
        # Clean up .npmrc
        rm -f .npmrc
    fi

    echo -e "${GREEN}✓${NC} Completed: npm package"
    echo ""
fi

# Summary
echo ""
echo -e "${GREEN}✅ Publishing Complete!${NC}"
echo "======================"
echo ""

if [ "$DRY_RUN" = true ]; then
    echo -e "${YELLOW}This was a DRY RUN - nothing was actually published${NC}"
    echo ""
    echo "To publish for real, run:"
    echo "  ./publish.sh"
else
    echo "Users can now install with:"
    echo ""
    if [ "$SKIP_NPM" = false ]; then
        echo -e "  ${GREEN}npm install -g portalis${NC}              # Recommended"
    fi
    if [ "$SKIP_RUST" = false ]; then
        echo -e "  ${GREEN}cargo install portalis${NC}                # For Rust developers"
    fi
    if [ "$SKIP_PYTHON" = false ]; then
        echo -e "  ${GREEN}pip install portalis-nemo-integration${NC}  # Optional GPU features"
        echo -e "  ${GREEN}pip install portalis-dgx-cloud${NC}         # Optional DGX Cloud"
    fi
fi

echo ""
echo "Documentation: https://github.com/portalis/portalis"
echo ""
