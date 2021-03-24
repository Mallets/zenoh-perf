all:
	gcc -O2 -Wall mqtt_pub_thr.c -lpaho-mqtt3a -pthread
	gcc -O2 -Wall mqtt_sub_thr.c -lpaho-mqtt3a -pthread

clean:
	rm mqtt_pub_thr mqtt_sub_thr