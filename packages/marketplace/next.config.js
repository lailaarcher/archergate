/** @type {import('next').NextConfig} */
const nextConfig = {
    async rewrites() {
        return [
            {
                source: '/sdk',
                destination: '/sdk.html',
            },
        ];
    },
};

module.exports = nextConfig;
