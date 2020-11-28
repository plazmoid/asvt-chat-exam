#include <stdio.h>
#include <unistd.h>
#include <sys/poll.h>
#include <time.h>

int main (void)
{
	struct pollfd fds[2];
	int ret;
	char msg[128];

	sprintf(msg, "hello, poll");

	// watch stdin for input
	fds[0].fd = STDIN_FILENO;
	fds[0].events = POLLIN;

	// watch stdout for ability to write
	fds[1].fd = STDOUT_FILENO;
	fds[1].events = POLLOUT;

	while(1) {
		ret = poll(fds, 2, 0);

		if (ret == -1) {
			perror ("poll");
			return 1;
		}

		if (fds[0].revents & POLLIN) {
			scanf("%s", msg);
		}
			
		if (fds[1].revents & POLLOUT) {
			printf("%s\n", msg);
		}
		sleep(2);
	}

	return 0;

}
