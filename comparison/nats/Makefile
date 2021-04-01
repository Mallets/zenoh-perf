# NATS-go version 1.10

all:
	mkdir -p ./target
	go build -o target/nats_pub_thr throughput/nats_pub_thr.go
	go build -o target/nats_sub_thr throughput/nats_sub_thr.go

clean:
	rm -rf target