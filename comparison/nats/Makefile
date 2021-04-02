# NATS-go version 1.10

all:
	mkdir -p ./target
	go build -o target/nats_pub_thr throughput/nats_pub_thr.go
	go build -o target/nats_sub_thr throughput/nats_sub_thr.go
	go build -o target/nats_ping latency/nats_ping.go
	go build -o target/nats_pong latency/nats_pong.go

clean:
	rm -rf target