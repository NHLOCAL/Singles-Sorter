document.addEventListener('DOMContentLoaded', () => {
  // --- Mobile Nav Toggle ---
  const navToggle = document.querySelector('.nav-toggle');
  const mainNav = document.querySelector('.main-nav');

  if (navToggle && mainNav) {
    navToggle.addEventListener('click', () => {
      mainNav.classList.toggle('is-open');
      navToggle.classList.toggle('is-open');
      document.body.classList.toggle('nav-open');
    });
  }

  // --- Download Links ---
  const GITHUB_REPO = 'NHLOCAL/Singles-Sorter';
  const API_URL = `https://api.github.com/repos/${GITHUB_REPO}/releases/latest`;
  const FALLBACK_URL = `https://github.com/${GITHUB_REPO}/releases/latest`;

  const downloadLinks = {
    installer: document.getElementById('download-installer'),
    portable: document.getElementById('download-portable'),
    android: document.getElementById('download-android'),
    cli: document.getElementById('download-cli'),
  };

  async function updateDownloadLinks() {
    try {
      const response = await fetch(API_URL);
      if (!response.ok) {
        throw new Error(`GitHub API error: ${response.status}`);
      }
      const data = await response.json();
      const version = data.tag_name.replace('v', ''); // Remove 'v' prefix, e.g., "14.0.0"
      
      if (!version) {
        throw new Error('Version number not found in API response.');
      }

      const baseUrl = `https://github.com/${GITHUB_REPO}/releases/download/v${version}`;

      // Construct the final URLs based on the exact file naming convention
      const urls = {
        installer: `${baseUrl}/Singles-Sorter-Installer-AI-${version}.exe`,
        portable: `${baseUrl}/Singles-Sorter-Portable-AI-${version}.zip`,
        android: `${baseUrl}/Singles-Sorter-${version}.apk`,
        cli: `${baseUrl}/Singles-Sorter-cli-ai-${version}.zip`,
      };

      // Update the href attribute for each download button
      if (downloadLinks.installer) downloadLinks.installer.href = urls.installer;
      if (downloadLinks.portable) downloadLinks.portable.href = urls.portable;
      if (downloadLinks.android) downloadLinks.android.href = urls.android;
      if (downloadLinks.cli) downloadLinks.cli.href = urls.cli;

    } catch (error) {
      console.error('Failed to fetch latest version from GitHub:', error);
      // In case of an error, point all links to the main releases page as a fallback
      for (const key in downloadLinks) {
        if (downloadLinks[key]) {
          downloadLinks[key].href = FALLBACK_URL;
        }
      }
    }
  }

  updateDownloadLinks();
});