import docker
import time
import json

def calculate_average_cpu_usage(container_name, duration=10):
    client = docker.from_env()
    container = client.containers.get(container_name)
    stats = container.stats(stream=True)

    cpu_percentages = []
    start_time = time.time()

    try:
        for stat in stats:
            # Decode the byte string to a dictionary
            stat = json.loads(stat.decode('utf-8'))
            
            # Calculate elapsed time
            elapsed_time = time.time() - start_time
            if elapsed_time > duration:
                break

            # Check if 'system_cpu_usage' exists in both 'cpu_stats' and 'precpu_stats'
            if "system_cpu_usage" not in stat["cpu_stats"] or "system_cpu_usage" not in stat["precpu_stats"]:
                continue
            
            # CPU usage calculation
            cpu_delta = stat["cpu_stats"]["cpu_usage"]["total_usage"] - stat["precpu_stats"]["cpu_usage"]["total_usage"]
            system_cpu_delta = stat["cpu_stats"]["system_cpu_usage"] - stat["precpu_stats"]["system_cpu_usage"]
            num_cpus = len(stat["cpu_stats"]["cpu_usage"].get("percpu_usage", []))

            if system_cpu_delta > 0 and cpu_delta > 0:
                cpu_percentage = (cpu_delta / system_cpu_delta) * num_cpus * 100.0
                cpu_percentages.append(cpu_percentage)

            time.sleep(1)  # Poll every 1 second

    except KeyboardInterrupt:
        print("Interrupted by user")

    # Calculate average CPU usage
    if cpu_percentages:
        average_cpu_usage = sum(cpu_percentages) / len(cpu_percentages)
        print(f"Average CPU usage for container '{container_name}' over {duration} seconds: {average_cpu_usage:.2f}%")
    else:
        print("No CPU usage data collected.")

if __name__ == "__main__":
    container_name = "mdd-rest-gateway"
    calculate_average_cpu_usage(container_name, duration=290)
