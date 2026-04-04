// Simple event tracking endpoint
// Logs downloads and signups to stdout (pipe to a log aggregator)
// Deploy as serverless function alongside sdk-beta.js

export default async function handler(req, res) {
    if (req.method !== 'POST') {
        return res.status(405).json({ error: 'Method not allowed' });
    }

    const { event, platform, ts } = req.body || {};

    // Log to stdout (Vercel/Netlify capture this automatically)
    console.log(JSON.stringify({
        event: event || 'unknown',
        platform: platform || null,
        ts: ts || Date.now(),
        ip: req.headers['x-forwarded-for'] || req.socket?.remoteAddress || null,
        ua: req.headers['user-agent'] || null,
        ref: req.headers['referer'] || null,
    }));

    return res.status(200).json({ ok: true });
}
