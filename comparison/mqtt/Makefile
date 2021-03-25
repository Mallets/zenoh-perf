all:
	mkdir -p ./target
	gcc -O2 -Wall throughput/mqtt_pub_thr.c -lpaho-mqtt3a -pthread -o target/mqtt_pub_thr
	gcc -O2 -Wall throughput/mqtt_sub_thr.c -lpaho-mqtt3a -pthread -o target/mqtt_sub_thr
	gcc -O2 -Wall latency/mqtt_ping.c -lpaho-mqtt3a -pthread -o target/mqtt_ping
	gcc -O2 -Wall latency/mqtt_pong.c -lpaho-mqtt3a -pthread -o target/mqtt_pong
	gcc -O2 -Wall overhead/mqtt_overhead.c -lpaho-mqtt3a -pthread -o target/mqtt_overhead

clean:
	rm -rf target