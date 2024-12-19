def generate_jobs(start, end, step, output_file):
    job_template = """  process_songs_{start}-{end}:
    runs-on: ubuntu-latest

    steps:
      - name: Checkout code
        uses: actions/checkout@v4

      - name: Set up Python
        uses: actions/setup-python@v5
        with:
          python-version: '3.11'

      - name: Install dependencies
        run: |
          python -m pip install --upgrade pip
          pip install google-generativeai

      - name: Run Python Script for lines {start}-{end}
        env:
          GEMINI_API_KEY: ${{{{ secrets.GEMINI_API_KEY }}}}
        run: |
          python machine-learn/scrape_data/level0/Gemini-synthetic-v2/1-gemini_tagging-api.py {start} {end}

      - name: Upload Artifact
        uses: actions/upload-artifact@v4
        with:
          name: song-tags-output-{start}-{end}
          path: tagged_songs.json
"""

    jobs = []

    for i in range(start, end, step):
        job_start = i
        job_end = i + step - 1
        jobs.append(job_template.format(start=job_start, end=job_end))

    with open(output_file, 'w') as f:
        f.write("\n".join(jobs))

# Generate jobs for ranges 1000 to 5000 with a step of 1000
generate_jobs(60001, 73370, 2000, 'jobs_output.txt')
