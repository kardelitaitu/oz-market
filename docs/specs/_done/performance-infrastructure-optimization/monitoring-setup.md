# Performance Monitoring Setup

## Metrics Endpoint

The application exposes Prometheus-compatible metrics at `/metrics`:

```
curl http://localhost:3000/metrics
```

## Available Metrics

### Database Connection Pool
- `database_connections_total`: Total active connections
- `database_connections_idle`: Number of idle connections
- `database_connections_utilization_percent`: Pool utilization percentage

### Runtime Information
- `runtime_worker_threads`: Configured tokio worker threads
- `runtime_cpu_cores`: Available CPU cores

### Cache Performance
- `cache_listing_entries`: Current cached listings
- `cache_listing_utilization_percent`: Listing cache utilization %
- `cache_search_entries`: Current cached searches
- `cache_search_utilization_percent`: Search cache utilization %
- `memory_cache_estimated_mb`: Estimated cache memory usage

## Monitoring Setup

### Prometheus Configuration
Add to `prometheus.yml`:

```yaml
scrape_configs:
  - job_name: 'oz-market-server'
    static_configs:
      - targets: ['localhost:3000']
    metrics_path: '/metrics'
```

### Grafana Dashboard

Import the following dashboard JSON for basic monitoring:

```json
{
  "dashboard": {
    "title": "Marketplace Server Performance",
    "panels": [
      {
        "title": "Database Connections",
        "type": "graph",
        "targets": [
          {
            "expr": "database_connections_total",
            "legendFormat": "Total"
          },
          {
            "expr": "database_connections_idle",
            "legendFormat": "Idle"
          }
        ]
      },
      {
        "title": "Cache Utilization",
        "type": "bargauge",
        "targets": [
          {
            "expr": "cache_listing_utilization_percent",
            "title": "Listing Cache"
          },
          {
            "expr": "cache_search_utilization_percent",
            "title": "Search Cache"
          }
        ]
      }
    ]
  }
}
```

## Key Alerts

- Database connection utilization > 80%
- Cache utilization > 90%
- Memory usage > system limits
- Worker threads at minimum (potential performance bottleneck)