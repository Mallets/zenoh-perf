all:
	gcc -O2 -Wall -o zmq_pub_thr zmq_pub_thr.c -lzmq
	gcc -O2 -Wall -o zmq_sub_thr zmq_sub_thr.c -lzmq -pthread

clean:
	rm zmq_pub_thr zmq_sub_thr