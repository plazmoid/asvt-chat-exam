#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <stdbool.h>
#include <string.h>
#include <errno.h>
#include <sys/stat.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <fcntl.h>
#include <time.h>
#include <arpa/inet.h>

#define HOST "3.9.16.40"
#define PORT 81
#define MAX_BUF_SIZE 256

//*******************SERVICE FUNCTIONS******************
void err(char *msg, const char *arg, bool critical) {
    if(msg == NULL || strlen(msg) == 0) {
        msg = strerror(errno);
		if (!strcmp(msg, "Success")) {
			return;
		}
    }
    if (arg != NULL) {
        fprintf(stderr, "%s, '%s'\n", msg, arg);
    } else {
        fprintf(stderr, "%s\n", msg);
    }
    if(critical) {
        exit(-1);
    }
}

// handy error checker
int errwrap(int ret) {
    if(ret == -1) {
		// critical, show error and exit
        err(NULL, NULL, true);
        return -1;
    } else {
		// show error and continue execution
        err(NULL, NULL, false);
        return ret;
    }
}

int sock_init() {
	struct sockaddr_in serv_addr;
    int sock_fd = errwrap(socket(AF_INET, SOCK_STREAM, 0));
    serv_addr.sin_family = AF_INET;
    serv_addr.sin_port = htons(PORT);
	errwrap(inet_pton(AF_INET, HOST, &serv_addr.sin_addr));
	errwrap(connect(sock_fd, (struct sockaddr *)&serv_addr, sizeof(serv_addr)));
	return sock_fd;
}
//*******************SERVICE FUNCTIONS******************

//*******************COMMANDS HANDLING******************


// Handlers are functions that convert cmd_args to the protocol format and send data to the server
//
// Example:
// stdin: sendmsg alex "hi, alex!";
// result (server-acceptable string): SENDMSG|TO:alex|MSG:hi, alex
//
// Handlers may only send data
// because you'll receive answers from server with select()

int send_buf(int sock_fd, char* buf) {
	return errwrap(send(sock_fd, buf, strlen(buf), 0));
}

int handle_ECHO(int sock_fd) {
	char* cmd = "ECHO";
	char cmd_args[MAX_BUF_SIZE];
	char buf[MAX_BUF_SIZE];
	scanf("%s", cmd_args);
	sprintf(buf, "%s %s", cmd, cmd_args);
	return send_buf(sock_fd, buf);
}

int handle_PING(int sock_fd) {
	char* cmd = "PING";
	return send_buf(sock_fd, cmd);
}

int handle_USERS(int sock_fd) {
	char* cmd = "USERS";
	return send_buf(sock_fd, cmd);
}

int handle_HELP(int sock_fd) {
	char* cmd = "HELP";
	return send_buf(sock_fd, cmd);
}

void main_loop() {
	int sock_fd = sock_init();
	int user_input = 0;
	char result[MAX_BUF_SIZE];
	char* help = 
		"Choose an option:\n\n"
		"1. Echo <msg>\n"
		"2. Ping\n"
		"3. Show users\n"
		"4. Show help\n\n";

	while(1) {
		printf(help);
		scanf("%d", &user_input);
		switch(user_input) {
			case 1: {
				handle_ECHO(sock_fd);
				break;
			}
			case 2: {
				handle_PING(sock_fd);
				break;
			}
			case 3: {
				handle_USERS(sock_fd);
				break;
			}
			case 4: {
				handle_HELP(sock_fd);
				break;
			}
			default: {
				printf("Wrong option");
				continue;
			}
		}
		memset(result, 0, sizeof(result));
		errwrap(recv(sock_fd, result, MAX_BUF_SIZE, 0));
		printf("Response: \n%s\n", result);
	}
    close(sock_fd);
}
//*******************COMMANDS HANDLING******************

int main(int argc, char **argv) {
	main_loop();
    return 0;
}
