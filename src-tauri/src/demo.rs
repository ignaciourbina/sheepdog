use rusqlite::{params, Connection, Result};

use crate::db;

struct DemoVenv {
    project: &'static str,
    venv_name: &'static str,
    python: &'static str,
    config_files: &'static str,
    packages: &'static [(&'static str, &'static str)],
}

const DEMO_VENVS: &[DemoVenv] = &[
    DemoVenv {
        project: "/home/user/projects/web-dashboard",
        venv_name: ".venv",
        python: "3.13.5",
        config_files: "requirements.txt,pyproject.toml",
        packages: &[
            ("django", "5.1.4"), ("djangorestframework", "3.15.2"), ("celery", "5.4.0"),
            ("redis", "5.2.1"), ("psycopg2-binary", "2.9.10"), ("gunicorn", "23.0.0"),
            ("whitenoise", "6.8.2"), ("django-cors-headers", "4.6.0"), ("dj-database-url", "2.3.0"),
            ("python-decouple", "3.8"), ("requests", "2.32.3"), ("boto3", "1.35.90"),
            ("Pillow", "11.1.0"), ("django-storages", "1.14.4"), ("sentry-sdk", "2.19.2"),
            ("pytest", "8.3.4"), ("pytest-django", "4.9.0"), ("factory-boy", "3.3.1"),
            ("coverage", "7.6.10"),
        ],
    },
    DemoVenv {
        project: "/home/user/projects/ml-pipeline",
        venv_name: "venv",
        python: "3.12.3",
        config_files: "requirements.txt,setup.py",
        packages: &[
            ("numpy", "2.2.1"), ("pandas", "2.2.3"), ("scikit-learn", "1.6.1"),
            ("torch", "2.5.1"), ("transformers", "4.47.1"), ("datasets", "3.2.0"),
            ("accelerate", "1.2.1"), ("wandb", "0.19.1"), ("matplotlib", "3.10.0"),
            ("seaborn", "0.13.2"), ("scipy", "1.15.0"), ("tqdm", "4.67.1"),
            ("tensorboard", "2.18.0"), ("safetensors", "0.4.5"), ("tokenizers", "0.21.0"),
            ("pillow", "11.1.0"), ("pyyaml", "6.0.2"), ("hydra-core", "1.3.2"),
            ("omegaconf", "2.3.0"), ("einops", "0.8.0"), ("timm", "1.0.12"),
            ("lightning", "2.4.0"), ("torchvision", "0.20.1"),
        ],
    },
    DemoVenv {
        project: "/home/user/projects/data-analysis",
        venv_name: ".venv",
        python: "3.13.5",
        config_files: "requirements.txt",
        packages: &[
            ("pandas", "2.2.3"), ("numpy", "2.2.1"), ("matplotlib", "3.10.0"),
            ("seaborn", "0.13.2"), ("jupyter", "1.1.1"), ("notebook", "7.3.2"),
            ("scipy", "1.15.0"), ("statsmodels", "0.14.4"), ("plotly", "5.24.1"),
            ("openpyxl", "3.1.5"), ("xlsxwriter", "3.2.0"),
        ],
    },
    DemoVenv {
        project: "/home/user/projects/api-gateway",
        venv_name: ".venv",
        python: "3.14.0",
        config_files: "pyproject.toml",
        packages: &[
            ("fastapi", "0.115.6"), ("uvicorn", "0.34.0"), ("pydantic", "2.10.4"),
            ("httpx", "0.28.1"), ("sqlalchemy", "2.0.36"), ("alembic", "1.14.1"),
            ("python-jose", "3.3.0"), ("passlib", "1.7.4"), ("python-multipart", "0.0.20"),
            ("redis", "5.2.1"), ("celery", "5.4.0"), ("sentry-sdk", "2.19.2"),
            ("structlog", "24.4.0"), ("prometheus-client", "0.21.1"),
        ],
    },
    DemoVenv {
        project: "/home/user/projects/otree-experiment",
        venv_name: "venv",
        python: "3.12.3",
        config_files: "requirements.txt",
        packages: &[
            ("otree", "5.11.4"), ("django", "4.2.16"), ("channels", "4.2.0"),
            ("daphne", "4.1.2"), ("huey", "2.5.2"), ("psycopg2-binary", "2.9.10"),
            ("sentry-sdk", "2.19.2"), ("whitenoise", "6.8.2"), ("numpy", "2.2.1"),
        ],
    },
    DemoVenv {
        project: "/home/user/projects/otree-experiment",
        venv_name: ".venv",
        python: "3.13.5",
        config_files: "requirements.txt",
        packages: &[
            ("otree", "6.0.13"), ("django", "5.1.4"), ("channels", "4.2.0"),
            ("daphne", "4.1.2"), ("huey", "2.5.2"), ("psycopg2-binary", "2.9.10"),
            ("numpy", "2.2.1"), ("scipy", "1.15.0"), ("pandas", "2.2.3"),
        ],
    },
    DemoVenv {
        project: "/home/user/projects/cli-toolbox",
        venv_name: "venv",
        python: "3.13.5",
        config_files: "pyproject.toml,setup.cfg",
        packages: &[
            ("click", "8.1.8"), ("rich", "13.9.4"), ("typer", "0.15.1"),
            ("httpx", "0.28.1"), ("pydantic", "2.10.4"), ("toml", "0.10.2"),
        ],
    },
    DemoVenv {
        project: "/home/user/projects/scraper-service",
        venv_name: ".venv",
        python: "3.12.3",
        config_files: "requirements.txt",
        packages: &[
            ("scrapy", "2.12.0"), ("beautifulsoup4", "4.12.3"), ("selenium", "4.27.1"),
            ("playwright", "1.49.1"), ("httpx", "0.28.1"), ("parsel", "1.9.1"),
            ("lxml", "5.3.0"), ("fake-useragent", "2.0.3"), ("tqdm", "4.67.1"),
            ("sqlite-utils", "3.38"), ("datasette", "0.65.1"),
        ],
    },
    DemoVenv {
        project: "/home/user/research/survey-analysis",
        venv_name: "venv",
        python: "3.13.5",
        config_files: "requirements.txt",
        packages: &[
            ("pandas", "2.2.3"), ("numpy", "2.2.1"), ("pingouin", "0.5.5"),
            ("scipy", "1.15.0"), ("statsmodels", "0.14.4"), ("matplotlib", "3.10.0"),
            ("seaborn", "0.13.2"), ("tabulate", "0.9.0"), ("openpyxl", "3.1.5"),
        ],
    },
    DemoVenv {
        project: "/home/user/research/agent-simulation",
        venv_name: ".venv",
        python: "3.14.0",
        config_files: "pyproject.toml",
        packages: &[
            ("mesa", "3.1.1"), ("numpy", "2.2.1"), ("matplotlib", "3.10.0"),
            ("networkx", "3.4.2"), ("scipy", "1.15.0"), ("pandas", "2.2.3"),
            ("tqdm", "4.67.1"), ("jupyter", "1.1.1"), ("seaborn", "0.13.2"),
            ("plotly", "5.24.1"),
        ],
    },
    DemoVenv {
        project: "/home/user/projects/chatbot-service",
        venv_name: ".venv",
        python: "3.14.0",
        config_files: "pyproject.toml,requirements.txt",
        packages: &[
            ("openai", "1.58.1"), ("anthropic", "0.42.0"), ("langchain", "0.3.14"),
            ("chromadb", "0.5.23"), ("tiktoken", "0.8.0"), ("fastapi", "0.115.6"),
            ("uvicorn", "0.34.0"), ("pydantic", "2.10.4"), ("redis", "5.2.1"),
            ("python-dotenv", "1.0.1"), ("structlog", "24.4.0"),
        ],
    },
    DemoVenv {
        project: "/home/user/teaching/intro-to-python",
        venv_name: "venv",
        python: "3.13.5",
        config_files: "requirements.txt",
        packages: &[
            ("jupyter", "1.1.1"), ("notebook", "7.3.2"), ("numpy", "2.2.1"),
            ("matplotlib", "3.10.0"), ("pandas", "2.2.3"),
        ],
    },
    DemoVenv {
        project: "/home/user/teaching/data-science-workshop",
        venv_name: "venv",
        python: "3.13.5",
        config_files: "requirements.txt,environment.yml",
        packages: &[
            ("jupyter", "1.1.1"), ("numpy", "2.2.1"), ("pandas", "2.2.3"),
            ("scikit-learn", "1.6.1"), ("matplotlib", "3.10.0"), ("seaborn", "0.13.2"),
            ("plotly", "5.24.1"), ("xgboost", "2.1.3"), ("lightgbm", "4.5.0"),
            ("shap", "0.46.0"),
        ],
    },
    DemoVenv {
        project: "/home/user/projects/pdf-processor",
        venv_name: "venv",
        python: "3.12.3",
        config_files: "requirements.txt",
        packages: &[
            ("pypdf", "5.1.0"), ("reportlab", "4.2.5"), ("Pillow", "11.1.0"),
            ("ocrmypdf", "16.7.0"), ("click", "8.1.8"), ("tqdm", "4.67.1"),
        ],
    },
    DemoVenv {
        project: "/home/user/projects/monitoring-stack",
        venv_name: ".venv",
        python: "3.14.0",
        config_files: "pyproject.toml",
        packages: &[
            ("prometheus-client", "0.21.1"), ("grafana-api", "1.0.3"),
            ("requests", "2.32.3"), ("pyyaml", "6.0.2"), ("click", "8.1.8"),
            ("structlog", "24.4.0"), ("python-dotenv", "1.0.1"), ("schedule", "1.2.2"),
        ],
    },
];

pub fn populate_demo_data(conn: &Connection) -> Result<()> {
    db::clear_all(conn)?;

    let scanned_at = "2026-03-27 19:30:00";

    for (index, demo) in DEMO_VENVS.iter().enumerate() {
        let venv_path = format!("{}/{}", demo.project, demo.venv_name);
        // Give demo environments deterministic, varied sizes so the dashboard
        // and CLI can exercise sorting and human-readable formatting.
        let size_bytes = (120 + index as i64 * 73) * 1024 * 1024;
        conn.execute(
            "INSERT INTO venvs (path, project_path, python_version, python_executable, venv_name, last_modified, scanned_at, size_bytes, config_files)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                venv_path,
                demo.project,
                demo.python,
                format!("{}/{}/bin/python", demo.project, demo.venv_name),
                demo.venv_name,
                "2026-03-15 10:30",
                scanned_at,
                size_bytes,
                demo.config_files,
            ],
        )?;
        let venv_id = conn.last_insert_rowid();

        for (name, version) in demo.packages {
            conn.execute(
                "INSERT INTO packages (venv_id, name, version, summary) VALUES (?1, ?2, ?3, NULL)",
                params![venv_id, name, version],
            )?;
        }
    }

    Ok(())
}
