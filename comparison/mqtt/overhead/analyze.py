import json
import argparse


def read_json(filename):
    f = open(filename, 'r')
    data = json.load(f)
    f.close()
    return data


def main(datafile):
    data = read_json(datafile)

    total_frame_size = 0
    total_ip_size = 0
    total_tcp_size = 0
    total_mqtt_size = 0
    total_mqtt_data_size = 0
    total_payload = 0

    for p in data:
        total_frame_size +=  int(p['_source']['layers']['frame.len'][0])
        total_ip_size += int(p['_source']['layers']['ip.len'][0])
        if 'tcp.pdu.size' in p['_source']['layers']:
            total_tcp_size += int(p['_source']['layers']['tcp.pdu.size'][0])
        if 'mqtt.len' in p['_source']['layers']:
            total_mqtt_size += int(p['_source']['layers']['mqtt.len'][0])
        if 'mqtt.msgtype' in p['_source']['layers']:
            if int(p['_source']['layers']['mqtt.msgtype'][0]) == 3: #MQTT Publish Message
                total_payload += int(len(p['_source']['layers']['mqtt.msg'][0])/2)
                total_mqtt_data_size += int(p['_source']['layers']['mqtt.len'][0])
            elif int(p['_source']['layers']['mqtt.msgtype'][0]) == 4: #MQTT Puback Message 
                total_mqtt_data_size += int(p['_source']['layers']['mqtt.len'][0])

    print('protocol,total_wire,total_ip,total_tcp,total_mqtt,data_mqtt,payload')
    print(f"mqtt,{total_frame_size},{total_ip_size},{total_tcp_size},{total_mqtt_size},{total_mqtt_data_size},{total_payload}")


if __name__=='__main__':
    parser = argparse.ArgumentParser(description='MQTT Overhead Analyzer')
    parser.add_argument('data', help='JSON data file to be analyzed')
    args = parser.parse_args()
    main(args.data)