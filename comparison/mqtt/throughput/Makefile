all:
	gcc -O2 -Wall -o mqtt_pub_thr  mqtt-pub-thr.c -lpaho-mqtt3a -pthread
	gcc -O2 -Wall -o mqtt_sub_thr  mqtt-sub-thr.c -lpaho-mqtt3a -pthread

clean:
	rm mqtt_pub_thr mqtt_sub_thr