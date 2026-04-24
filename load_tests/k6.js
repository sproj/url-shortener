import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend } from 'k6/metrics';

// ---------------------------------------------------------------------------
// Custom metrics
// These become Prometheus metrics under the k6_ prefix after the remote write.
// Defining them explicitly lets us add per-scenario semantics.
// ---------------------------------------------------------------------------
const shortenErrorRate = new Rate('shorten_errors');
const redirectErrorRate = new Rate('redirect_errors');
const shortenDuration = new Trend('shorten_duration_ms', true);
const redirectDuration = new Trend('redirect_duration_ms', true);

// ---------------------------------------------------------------------------
// Test configuration
// The BASE_URL env var lets us switch between internal (default) and external
// without changing the script — useful for Phase 2 when CloudFront is in place.
// ---------------------------------------------------------------------------
const BASE_URL = __ENV.BASE_URL || 'http://url-shortener-service.url-shortener.svc.cluster.local:8080';

// A small pool of target URLs to shorten — varied enough to avoid any
// application-level caching of the input, realistic enough to be meaningful.
const TARGET_URLS = [
    'https://example.com/page/one',
    'https://example.com/page/two',
    'https://example.com/page/three',
    'https://docs.example.com/getting-started',
    'https://blog.example.com/post/load-testing-with-k6',
];

// ---------------------------------------------------------------------------
// Load profile — a gentle baseline ramp, not a stress test.
// Goal today: establish normal behaviour metrics, not find the breaking point.
//
// Stages:
//   0→5 VU  over 30s  — warm up, let the JIT and connection pools settle
//   5 VU    for 2m    — steady state, this is the baseline we care about
//   5→0 VU  over 30s  — cool down
//
// Total duration: ~3 minutes. Easy to re-run.
// ---------------------------------------------------------------------------
export const options = {
    stages: [
        { duration: '30s', target: 5 },
        { duration: '2m', target: 5 },
        { duration: '30s', target: 0 },
    ],

    // Thresholds define pass/fail for the job. These are deliberately loose
    // for a first baseline run — tighten them once you know your numbers.
    thresholds: {
        // 95th percentile shorten latency under 500ms
        'shorten_duration_ms': ['p(95)<500'],
        // 95th percentile redirect latency under 200ms (should be faster — Redis path)
        'redirect_duration_ms': ['p(95)<200'],
        // Error rates under 1%
        'shorten_errors': ['rate<0.01'],
        'redirect_errors': ['rate<0.01'],
        // Overall http error rate
        'http_req_failed': ['rate<0.01'],
    },

    // Push results to Prometheus via remote write.
    // The service address is the in-cluster DNS for kube-prometheus-stack.
    ext: {
        loadimpact: {
            projectID: 0,
        },
    },
};

// ---------------------------------------------------------------------------
// The main scenario: shorten a URL, then immediately redirect via the code.
// Each VU runs this in a loop for the duration of the test.
// ---------------------------------------------------------------------------
export default function () {
    const targetUrl = TARGET_URLS[Math.floor(Math.random() * TARGET_URLS.length)];

    // --- Write path: POST /shorten -------------------------------------------
    const shortenRes = http.post(
        `${BASE_URL}/shorten`,
        JSON.stringify({ long_url: targetUrl }),
        {
            headers: { 'Content-Type': 'application/json' },
            tags: { endpoint: 'shorten' },
        }
    );

    shortenDuration.add(shortenRes.timings.duration, { endpoint: 'shorten' });

    const shortenOk = check(shortenRes, {
        'shorten: status 201': (r) => r.status === 201,
        'shorten: has code in body': (r) => {
            try {
                return JSON.parse(r.body).code !== undefined;
            } catch {
                return false;
            }
        },
    });

    shortenErrorRate.add(!shortenOk);

    // If the shorten failed we can't do a meaningful redirect — skip and record.
    if (!shortenOk) {
        console.warn(`Shorten failed: status=${shortenRes.status} body=${shortenRes.body}`);
        sleep(1);
        return;
    }

    const code = JSON.parse(shortenRes.body).code;

    // Small pause between write and read — simulates a real user copying the
    // short link and then clicking it. Also prevents the two requests from
    // appearing as a single burst in the latency histograms.
    sleep(0.5);

    // --- Read path: GET /r/{code} --------------------------------------------
    // maxRedirects: 0 is critical — we want to measure *our* app returning the
    // redirect response, not the round-trip to the destination URL.
    const redirectRes = http.get(
        `${BASE_URL}/r/${code}`,
        {
            redirects: 0,
            tags: { endpoint: 'redirect' },
        }
    );

    redirectDuration.add(redirectRes.timings.duration, { endpoint: 'redirect' });

    const redirectOk = check(redirectRes, {
        'redirect: status 301 or 302': (r) => r.status === 301 || r.status === 302,
        'redirect: Location header present': (r) => r.headers['Location'] !== undefined,
    });

    redirectErrorRate.add(!redirectOk);

    if (!redirectOk) {
        console.warn(`Redirect failed: status=${redirectRes.status} code=${code}`);
    }

    // Pause between iterations — keeps VU behaviour realistic and prevents
    // a single VU from generating an unrealistic request rate.
    sleep(1);
}

// ---------------------------------------------------------------------------
// setup() runs once before the test, outside the VU loop.
// We use it to verify the app is reachable before starting load.
// A failed setup() aborts the test cleanly rather than flooding logs with errors.
// ---------------------------------------------------------------------------
export function setup() {
    console.log(`Target: ${BASE_URL}`);
    console.log('Verifying application health before starting load...');

    const res = http.get(`${BASE_URL}/health`);

    if (res.status !== 200) {
        throw new Error(`Health check failed: status=${res.status} — aborting test`);
    }

    console.log('Health check passed. Starting load test.');
}