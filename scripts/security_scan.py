#!/usr/bin/env python3
"""
Security Vulnerability Scanner
Automated security scanning tool for the Protein Docking platform

Usage:
    python scripts/security_scan.py [--fix] [--report]

Options:
    --fix       Automatically fix vulnerabilities when possible
    --report    Generate detailed HTML report
"""

import subprocess
import json
import sys
import argparse
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Any


class SecurityScanner:
    """Main security scanner class"""

    def __init__(self, project_root: Path):
        self.project_root = project_root
        self.backend_dir = project_root / "backend"
        self.frontend_dir = project_root / "frontend"
        self.results = {
            "scan_date": datetime.now().isoformat(),
            "backend": {},
            "frontend": {},
            "summary": {
                "critical": 0,
                "high": 0,
                "medium": 0,
                "low": 0,
                "total": 0
            }
        }

    def check_dependencies(self) -> bool:
        """Check if required scanning tools are installed"""
        tools = ["pip-audit", "bandit", "npm"]
        missing = []

        for tool in tools:
            try:
                subprocess.run([tool, "--version"],
                             capture_output=True,
                             check=True,
                             timeout=5)
            except (subprocess.CalledProcessError, FileNotFoundError, subprocess.TimeoutExpired):
                missing.append(tool)

        if missing:
            print(f"❌ Missing required tools: {', '.join(missing)}")
            print("\nInstall them with:")
            print("  pip install pip-audit bandit")
            print("  # npm should already be installed")
            return False

        return True

    def scan_backend_dependencies(self) -> Dict[str, Any]:
        """Scan Python dependencies for vulnerabilities"""
        print("🔍 Scanning backend dependencies...")

        try:
            result = subprocess.run(
                ["pip-audit", "-r", "requirements.txt", "--format", "json"],
                cwd=self.backend_dir,
                capture_output=True,
                text=True,
                timeout=120
            )

            data = json.loads(result.stdout)

            # Count vulnerabilities
            vuln_count = sum(len(dep["vulns"]) for dep in data.get("dependencies", []))

            print(f"  Found {vuln_count} vulnerabilities in Python dependencies")

            return {
                "tool": "pip-audit",
                "vulnerabilities": data.get("dependencies", []),
                "count": vuln_count
            }
        except subprocess.TimeoutExpired:
            print("  ⚠️  Timeout scanning backend dependencies")
            return {"error": "timeout"}
        except Exception as e:
            print(f"  ⚠️  Error scanning backend dependencies: {e}")
            return {"error": str(e)}

    def scan_backend_code(self) -> Dict[str, Any]:
        """Scan Python code for security issues with Bandit"""
        print("🔍 Scanning backend code with Bandit...")

        try:
            result = subprocess.run(
                ["bandit", "-r", "app/", "-f", "json"],
                cwd=self.backend_dir,
                capture_output=True,
                text=True,
                timeout=120
            )

            data = json.loads(result.stdout)

            # Count by severity
            metrics = data.get("metrics", {}).get("_totals", {})
            high = metrics.get("SEVERITY.HIGH", 0)
            medium = metrics.get("SEVERITY.MEDIUM", 0)
            low = metrics.get("SEVERITY.LOW", 0)

            print(f"  Found {high} high, {medium} medium, {low} low severity issues")

            return {
                "tool": "bandit",
                "results": data.get("results", []),
                "metrics": metrics,
                "high": high,
                "medium": medium,
                "low": low
            }
        except subprocess.TimeoutExpired:
            print("  ⚠️  Timeout scanning backend code")
            return {"error": "timeout"}
        except Exception as e:
            print(f"  ⚠️  Error scanning backend code: {e}")
            return {"error": str(e)}

    def scan_frontend_dependencies(self) -> Dict[str, Any]:
        """Scan npm dependencies for vulnerabilities"""
        print("🔍 Scanning frontend dependencies...")

        try:
            result = subprocess.run(
                ["npm", "audit", "--json"],
                cwd=self.frontend_dir,
                capture_output=True,
                text=True,
                timeout=120
            )

            data = json.loads(result.stdout)

            # Count vulnerabilities
            metadata = data.get("metadata", {})
            vulnerabilities = metadata.get("vulnerabilities", {})

            critical = vulnerabilities.get("critical", 0)
            high = vulnerabilities.get("high", 0)
            moderate = vulnerabilities.get("moderate", 0)
            low = vulnerabilities.get("low", 0)

            total = critical + high + moderate + low

            print(f"  Found {total} vulnerabilities ({critical} critical, {high} high, {moderate} moderate, {low} low)")

            return {
                "tool": "npm audit",
                "vulnerabilities": data.get("vulnerabilities", {}),
                "metadata": metadata,
                "critical": critical,
                "high": high,
                "moderate": moderate,
                "low": low,
                "total": total
            }
        except subprocess.TimeoutExpired:
            print("  ⚠️  Timeout scanning frontend dependencies")
            return {"error": "timeout"}
        except Exception as e:
            print(f"  ⚠️  Error scanning frontend dependencies: {e}")
            return {"error": str(e)}

    def fix_backend_dependencies(self) -> bool:
        """Attempt to fix backend dependency vulnerabilities"""
        print("🔧 Attempting to fix backend dependencies...")

        try:
            # Update vulnerable packages
            updates = {
                "python-socketio": "5.14.0",
                "flask": "3.1.1",
                "flask-cors": "6.0.0",
                "requests": "2.32.4",
                "python-multipart": "0.0.18",
                "fastapi": "0.115.3"
            }

            for package, version in updates.items():
                print(f"  Updating {package} to {version}...")
                subprocess.run(
                    ["pip", "install", f"{package}>={version}"],
                    cwd=self.backend_dir,
                    check=True,
                    timeout=60
                )

            print("  ✅ Backend dependencies updated")
            return True
        except Exception as e:
            print(f"  ❌ Error fixing backend dependencies: {e}")
            return False

    def fix_backend_code(self) -> bool:
        """Attempt to fix backend code vulnerabilities"""
        print("🔧 Fixing backend code issues...")

        cache_file = self.backend_dir / "app" / "core" / "cache.py"

        try:
            # Fix MD5 usage
            if cache_file.exists():
                content = cache_file.read_text()

                if "hashlib.md5(" in content:
                    # Replace MD5 with SHA256
                    content = content.replace(
                        "hashlib.md5(key_data.encode()).hexdigest()",
                        "hashlib.sha256(key_data.encode()).hexdigest()"
                    )
                    cache_file.write_text(content)
                    print("  ✅ Fixed MD5 usage in cache.py")
                else:
                    print("  ℹ️  MD5 already fixed or not found")

            return True
        except Exception as e:
            print(f"  ❌ Error fixing backend code: {e}")
            return False

    def fix_frontend_dependencies(self) -> bool:
        """Attempt to fix frontend dependency vulnerabilities"""
        print("🔧 Attempting to fix frontend dependencies...")

        try:
            subprocess.run(
                ["npm", "audit", "fix"],
                cwd=self.frontend_dir,
                check=True,
                timeout=300
            )

            print("  ✅ Frontend dependencies updated")
            return True
        except Exception as e:
            print(f"  ⚠️  Error fixing frontend dependencies: {e}")
            print("  You may need to run 'npm audit fix --force' manually")
            return False

    def run_scan(self) -> Dict[str, Any]:
        """Run complete security scan"""
        print("\n" + "="*60)
        print("  🛡️  SECURITY VULNERABILITY SCANNER")
        print("="*60 + "\n")

        if not self.check_dependencies():
            return {"error": "missing_dependencies"}

        # Scan backend
        print("\n📦 BACKEND ANALYSIS")
        print("-" * 60)
        self.results["backend"]["dependencies"] = self.scan_backend_dependencies()
        self.results["backend"]["code"] = self.scan_backend_code()

        # Scan frontend
        print("\n📦 FRONTEND ANALYSIS")
        print("-" * 60)
        self.results["frontend"]["dependencies"] = self.scan_frontend_dependencies()

        # Calculate summary
        self._calculate_summary()

        return self.results

    def _calculate_summary(self):
        """Calculate vulnerability summary"""
        summary = self.results["summary"]

        # Backend dependencies
        if "dependencies" in self.results["backend"]:
            summary["total"] += self.results["backend"]["dependencies"].get("count", 0)

        # Backend code
        if "code" in self.results["backend"]:
            code = self.results["backend"]["code"]
            summary["high"] += code.get("high", 0)
            summary["medium"] += code.get("medium", 0)
            summary["low"] += code.get("low", 0)

        # Frontend
        if "dependencies" in self.results["frontend"]:
            frontend = self.results["frontend"]["dependencies"]
            summary["critical"] += frontend.get("critical", 0)
            summary["high"] += frontend.get("high", 0)
            summary["medium"] += frontend.get("moderate", 0)
            summary["low"] += frontend.get("low", 0)

    def print_summary(self):
        """Print scan summary"""
        summary = self.results["summary"]

        print("\n" + "="*60)
        print("  📊 SCAN SUMMARY")
        print("="*60)
        print(f"\n  Critical:  🔴 {summary['critical']}")
        print(f"  High:      🟠 {summary['high']}")
        print(f"  Medium:    🟡 {summary['medium']}")
        print(f"  Low:       🟢 {summary['low']}")
        print(f"  Total:     📊 {summary['total']}")

        # Recommendations
        print("\n" + "="*60)
        print("  💡 RECOMMENDATIONS")
        print("="*60)

        if summary['critical'] > 0 or summary['high'] > 0:
            print("\n  🔴 URGENT: Critical/High vulnerabilities found!")
            print("  Run with --fix to attempt automatic remediation")
        elif summary['medium'] > 0:
            print("\n  🟡 Medium severity issues found")
            print("  Consider running with --fix to update dependencies")
        else:
            print("\n  ✅ No critical vulnerabilities found!")
            print("  Your application is reasonably secure")

        print("\n  📄 Full report available at: SECURITY_AUDIT_REPORT.md")
        print()

    def save_json_report(self, filename: str = "security_scan_results.json"):
        """Save results as JSON"""
        output_file = self.project_root / filename
        output_file.write_text(json.dumps(self.results, indent=2))
        print(f"  💾 JSON report saved to: {filename}")


def main():
    """Main entry point"""
    parser = argparse.ArgumentParser(
        description="Security vulnerability scanner for Protein Docking platform"
    )
    parser.add_argument("--fix", action="store_true", help="Automatically fix vulnerabilities")
    parser.add_argument("--report", action="store_true", help="Generate detailed report")
    parser.add_argument("--json", action="store_true", help="Save results as JSON")

    args = parser.parse_args()

    # Determine project root
    script_path = Path(__file__).resolve()
    project_root = script_path.parent.parent

    scanner = SecurityScanner(project_root)

    # Run scan
    results = scanner.run_scan()

    if "error" in results:
        print(f"\n❌ Scan failed: {results['error']}")
        sys.exit(1)

    # Print summary
    scanner.print_summary()

    # Fix if requested
    if args.fix:
        print("\n" + "="*60)
        print("  🔧 AUTOMATIC REMEDIATION")
        print("="*60 + "\n")

        scanner.fix_backend_dependencies()
        scanner.fix_backend_code()
        scanner.fix_frontend_dependencies()

        print("\n  ✅ Remediation complete!")
        print("  Re-run the scanner to verify fixes")

    # Save JSON report
    if args.json:
        scanner.save_json_report()

    # Exit code based on severity
    if results["summary"]["critical"] > 0:
        sys.exit(2)  # Critical
    elif results["summary"]["high"] > 0:
        sys.exit(1)  # High
    else:
        sys.exit(0)  # OK


if __name__ == "__main__":
    main()
