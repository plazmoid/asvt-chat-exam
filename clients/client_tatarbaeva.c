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
#include <sys/poll.h>
#include <termios.h>
#include <sys/ioctl.h>


#define HOST "3.9.16.135"
#define PORT 81
#define MAX_BUF_SIZE 256

#define ESC "\033"
#define set_display_atrib(color) 	printf(ESC "[%dm",color)
#define resetcolor() printf(ESC "[0m")

//Foreground Colours

#define F_BLACK 	30
#define F_RED		31
#define F_GREEN		32
#define F_YELLOW	33
#define F_BLUE		34
#define F_MAGENTA 	35
#define F_CYAN		36
#define F_WHITE		37

struct termios saved_attributes;

char** chat_buffer = NULL;
 
int window_color = 35;
int text_color = 36;
int user_color = 33;
int read_start = 0;
int read_end = 24;
int chat_pointer = 0;


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


void reset_input_mode (void)
{
   tcsetattr (STDIN_FILENO, TCSANOW, &saved_attributes);
}


void set_input_mode (void)
{
   struct termios tattr;

  /* Save the terminal attributes so we can restore them later. */
  tcgetattr (STDIN_FILENO, &saved_attributes);

  /* Set the funny terminal modes. */
  tcgetattr (STDIN_FILENO, &tattr);
  tattr.c_lflag &= ~(ICANON|ECHO); /* Clear ICANON and ECHO. */
  tcsetattr (STDIN_FILENO, TCSAFLUSH, &tattr);
}


int send_buf(int sock_fd, char* buf) {
	return errwrap(send(sock_fd, buf, strlen(buf), 0));
}


int LOGIN(int sock_fd, char* name, char* password) {
	char* cmd = "LOGIN|username=";
	char* dop_cmd = "|password=";
	
	char *result = malloc(strlen(cmd) + strlen(name) + strlen(dop_cmd) + strlen(password) + 1);
	stpcpy(result, cmd);
	strcat(result, name);
	strcat(result, dop_cmd);
	strcat(result, password);
	
	return send_buf(sock_fd, result);
}


int SEND(int sock_fd, char *name, char *message) {
	char* cmd = "SEND|username=";
	char* dop_cmd = "|msg=";
	
	char *result = malloc(strlen(cmd) + strlen(name) + strlen(dop_cmd) + strlen(message) + 1);
	stpcpy(result, cmd);
	strcat(result, name);
	strcat(result, dop_cmd);
	strcat(result, message);
	
	return send_buf(sock_fd, result);
}


int SEND_ALL(int sock_fd, char *message) {
	char* cmd = "SNDALL|msg=";
	
	char *result = malloc(strlen(cmd) + strlen(message) + 1);
	stpcpy(result, cmd);
	strcat(result, message);
	
	return send_buf(sock_fd, result);
}


int PING(int sock_fd) {
	char* cmd = "PING";
	return send_buf(sock_fd, cmd);
}


int USERS(int sock_fd) {
	char* cmd = "USERS";
	return send_buf(sock_fd, cmd);
}


void gotoxy(int x, int y) {
	printf("%c[%d;%df", 0x1b, y, x);
}


int getch() {
	int ch;
	struct termios oldt, newt;
	tcgetattr( STDIN_FILENO, &oldt );
	newt = oldt;
	newt.c_lflag &= ~( ICANON | ECHO );
	tcsetattr( STDIN_FILENO, TCSANOW, &newt );
	ch = getchar();
	tcsetattr( STDIN_FILENO, TCSANOW, &oldt );
	return ch;
}


void kbhit(int *x) {
	ioctl(0, FIONREAD, x);
}


//отрисовка со сдвигом о границы border
void print_shift(int x, int y, int border, char* msg){	
	int i = 1;
	int length;
	length = strlen(msg);
	char* delimiter = "\n";
	gotoxy(x,y);
	fflush(0);
	while(i <= length){		
		if (msg[i] == delimiter[0]){
			y = y + 1;
			gotoxy(x,y);	
		}
		else{
			printf("%c", msg[i]);
			fflush(0);
		}
		if (y == border){
			break;
		}
		i++;
	}
}


//зачистка со сдвигом окна с шириной width и высотой height
void delete_shift(int x, int y, int width, int height){	
	int x_start = x;
	int y_start = y;
	char* empt = " ";
	while(y < y_start + height){		
		if (x < x_start + width){
			gotoxy(x,y);
			printf(empt);
			fflush(0);
			x++;	
		}
		else{
			y++;
			x = x_start;
		}
	}
}


//отрисовка пользователей
void print_users(int sock_fd) {
	delete_shift(91, 2, 28, 24);
	set_display_atrib(user_color);
	char result[MAX_BUF_SIZE];
	USERS(sock_fd);
	memset(result, 0, sizeof(result));
	errwrap(recv(sock_fd, result, MAX_BUF_SIZE, 0));
	print_shift(91, 2, 26, result);
	if (sizeof(result) == 256){
		memset(result, 0, sizeof(result));
		errwrap(recv(sock_fd, result, MAX_BUF_SIZE, 0));
	}
	resetcolor();
}


//отрисовка сообщений чата
void print_messages(){
	delete_shift(3, 2, 85, 24);
	set_display_atrib(text_color);
	gotoxy(3,2);	
	int j = 0;
	int i = read_start;
	while (i < read_end){
		if (chat_buffer[i][1] == 'I'){
			set_display_atrib(text_color);
		}
		else if (chat_buffer[i][1] == '2'){
			set_display_atrib(user_color);
		}
		else if (chat_buffer[i][0] == '-'){
			set_display_atrib(F_RED);
		}
		gotoxy(3,2+j);
		printf(chat_buffer[i]);
		fflush(0);
		i++;
		j++;
	}
	resetcolor();
}


//отрисовка моих сообщений
void print_my_message(char *name, char *msg){
	char* text1 = "[I (to ";
	char* text2 = ")]: ";
	char* result = malloc(strlen(text1) + strlen(name) + strlen(text2) + strlen(msg) + 1);
	stpcpy(result, text1);
	strcat(result, name);
	strcat(result, text2);
	strcat(result, msg);
	add_message(result);
	print_messages();	
}


//добавление сообщения в буфер
void add_message(char* msg){
	if (chat_pointer >= read_end){
		if (chat_pointer < 50){		
		read_end++;
		read_start++;
		}	 
		else{
			buffer_shift();
			chat_pointer = 49;
		}
	}
		
	int i = 0;
	int j = 0;
	int x = 0;
	int length;
	length = strlen(msg);
	char* msg_char = "M";
	if (msg[0] == msg_char[0]){
		j = 8;
	}
	while (j < length){
		if (i == 85){
			chat_pointer++;
			x = 0;
			i = 0;
		}
		chat_buffer[chat_pointer][x] = msg[j];
		i++;
		j++;
		x++;
	}	
	chat_pointer++;
}


//очистка строки в буфере
void delete_string(int i){
	int j = 0;
	while (j < 85){
		chat_buffer[i][j] = 0;	
		j++;
	} 	
}


//сдвиг строк при переполнении буфера
void buffer_shift(){
	int i = 0;
	int j = 0;
	while (i<49){
		delete_string(i);
		while (j < 85){
			chat_buffer[i][j] = chat_buffer[i+1][j];
			j++;	
		} 	
		i++;
		j = 0;
	}		
	delete_string(i);
}


//отрисовка всего окошка
void print_all_window(int sock_fd) {
	set_display_atrib(window_color);
	int x = 1;
	int y = 1;	
	gotoxy(x, y);
	char* empt = " ";
	int i = 0;
	while (i < 3600){
		printf(empt);
		fflush(0);
		i++;
	}
	
	x = 1;
	y = 1;
	
	gotoxy(x, y);
	while (x != 121) {
		printf("#");
		fflush(0);
		x++;
	}
	
	x = 1;
	y = 1;
	while (y != 30) {
		gotoxy(x, y);
		printf("#");
		fflush(0);
		y++;
	}
	
	x = 120;
	y = 1;
	while (y != 30) {
		gotoxy(x, y);
		printf("#");
		fflush(0);
		y++;
	}
	
	x = 89;
	y = 1;
	while (y != 26) {
		gotoxy(x, y);
		printf("#");
		fflush(0);
		y++;
	}
	
	x = 1;
	y = 26;
	gotoxy(x, y);
	while (x != 121) {
		printf("#");
		fflush(0);
		x++;
	}
	
	x = 1;
	y = 30;
	gotoxy(x, y);
	while (x != 121) {
		printf("#");
		fflush(0);
		x++;
	}
	resetcolor();
	print_messages(sock_fd);
	print_users(sock_fd);
	gotoxy(3,27);
	fflush(0);
}


//затирание нижней части окна
void print_bottom_window() {
	set_display_atrib(window_color);
	int x = 1;
	int y = 27;	
	gotoxy(x, y);
	char* empt = " ";
	int i = 0;
	while (i < 480){
		printf(empt);
		fflush(0);
		i++;
	}
	
	//левая вертикальная до нижней гор границы
	x = 1;
	y = 27;
	while (y != 30) {
		gotoxy(x, y);
		printf("#");
		fflush(0);
		y++;
	}
	
	//правая вертикальная до нижней гор границы
	x = 120;
	y = 27;
	while (y != 30) {
		gotoxy(x, y);
		printf("#");
		fflush(0);
		y++;
	}
	
	//нижняя горизонтальная граница
	x = 1;
	y = 30;
	gotoxy(x, y);
	while (x != 121) {
		printf("#");
		fflush(0);
		x++;
	}
	
	gotoxy(3,27);
	fflush(0);
	resetcolor();
}


void clean_chat(){
	int i = 0;
	while (i<50){
	delete_string(i);
	i++;
	}
	read_start = 0;
	read_end = 24;
	chat_pointer = 0;
}


void main_loop(char* login, char* password) {
	set_input_mode();
	
	//выделение буфера для сообщений
	int j;	
	chat_buffer = (char**) malloc(51 * sizeof(char*));
	for (j = 0; j < 51; j++) {
		chat_buffer[j] = (char*) malloc(86 * sizeof(char));
	}
	//выделение буферов для текущего имени, сообщения и сохраненного имени пользователя 
	char* name = malloc(20*sizeof(char));
	char* save_name = malloc(20*sizeof(char));
	char* message = malloc(256*sizeof(char));
	
	int sock_fd = sock_init();
	int user_input = 0;
	char result[MAX_BUF_SIZE];
	
	struct pollfd fds[2];
	int ret;

	// watch stdin for input
	fds[0].fd = STDIN_FILENO;
	fds[0].events = POLLIN;

	// watch socket for ability to write
	fds[1].fd = sock_fd;
	fds[1].events = POLLIN;	
	
	sleep(1);
	
	LOGIN(sock_fd, login, password);
	memset(result, 0, sizeof(result));
	errwrap(recv(sock_fd, result, MAX_BUF_SIZE, 0));
	if (result[0] == '-'){
		printf("Неверный пароль\nПовторите попытку\n");
		goto _exit;
	}	
	
	sleep(1);
	
	print_all_window(sock_fd);
	
	sleep(1);
	
	int pos_x = 2;
	int pos_y = 27;
	
	time_t start, end;
    double elapsed;
	
	int command, flag_name, flag_message, use_save_name;	
	int chars, ch, ch2, n;	
	int i;

	i = 0;
	n = 0;
	use_save_name = 0;
	ch2 = 0;
	
	gotoxy(pos_x,pos_y);
	fflush(0);
	time(&start);
	while(1) {
		ret = poll(fds, 2, 0);

		if (ret == -1) {
			perror ("poll");
			return 1;
		}
		//приём ввода с клавиатуры
		if (fds[0].revents & POLLIN) {		
			kbhit(&chars);
			if (chars) {
				ch = getch();
				if (ch == 27){
					//обработка esc для выхода
					goto _exit;
				}
				else if (ch == 127){
					//обработка backspace для стирания введёных символов
					if (pos_x == 2 & pos_y == 27){
						continue;
					}
					if (pos_x == 2){
						pos_y--;
						pos_x = 118;
						
					}
					gotoxy(pos_x, pos_y);
					printf(" ");
					fflush(0);
					pos_x--;
					gotoxy(pos_x, pos_y);
					fflush(0);
					n--;
					if (flag_name == 1){
						i--;
						name[i] = 0;
						if (name[i-1] == 208 || name[i-1] == 209){
							i--;
							name[i] = 0;
						}
					}
					else if (flag_message == 1){
						i--;
						message[i] = 0;
						if (message[i-1] == 208 || message[i-1] == 209){
							i--;
							message[i] = 0;
						}
					}
				}
				else if (ch == 10){
					//обработка enter для отправки сообщения
					if (command == '*'){
						SEND_ALL(sock_fd, message);
						time(&start);
						memset(message, 0, 256);
					}
					else if (command == '@'){
						if (use_save_name == 0){
							memset(save_name, 0, 20);
							for (j = 0; j < 20; j++) {
								save_name[j] = name[j];
							}
						}
						else{
							for (j = 0; j < 20; j++) {
								name[j] = save_name[j];
							}
						}						
						SEND(sock_fd, name, message);
						
						memset(result, 0, sizeof(result));
						errwrap(recv(sock_fd, result, MAX_BUF_SIZE, 0));
						
						if (result[0] != '-'){
							print_my_message(name, message);
						}
						else {
							add_message(result);
							print_messages();
						}
						time(&start);
						memset(name, 0, 20);
						memset(message, 0, 256);
					}
					print_bottom_window();
					flag_message = 0;
					command = 0;
					n = 0;
					i = 0;
					pos_x = 2;
					pos_y = 27;
					gotoxy(pos_x, pos_y);
					fflush(0);
				}
				else if (ch == 45 & n == 0){
					//обработка минуса для скроллинга вверх
					if (read_start != 0){
						read_start--;
						read_end--;
						print_messages();
					}
				}
				else if (ch == 43 & n == 0){
					//обработка плюса для скроллинга вниз
					if (chat_pointer > 24 & read_end < chat_pointer){
						read_start++;
						read_end++;
						print_messages();
					}
				}
				else if (ch == 49 & n == 0){
					//обработка 1 для смены цвета рамки
					if (window_color == 31){
						window_color = 37;
					}
					else {
						window_color--;
					}
					print_all_window(sock_fd);
					time(&start);
				}
				else if (ch == 50 & n == 0){
					//обработка 2 для смены моего цвета(в наборе текста и сообщений в чате)
					if (text_color == 31){
						text_color = 37;
					}
					else {
						text_color--;
					}
					print_all_window(sock_fd);
					time(&start);
				}
				else if (ch == 51 & n == 0){
					//обработка 3 для смены цвета других пользователей(окно пользователей и сообщения пользователей)
					if (user_color == 31){
						user_color = 37;
					}
					else {
						user_color--;
					}
					print_all_window(sock_fd);
					time(&start);
				}
				else if (ch == 9){
					clean_chat();
					print_all_window(sock_fd);
				}
				else if ((ch >= 32 & ch <= 126) || ch == 208 || ch == 209){
					//обработка печатаемых символов
					//проверка длины сообщения
					if (flag_name == 1 & i == 20){
						if (ch != ' '){
							continue;
						}
					}
					else if (flag_message == 1 & n == 232){
						continue;
					}
					//печать
					set_display_atrib(text_color);
					if (pos_x < 118 & pos_y < 30){
						pos_x++;
						gotoxy(pos_x, pos_y);
						if (ch == 208 || ch == 209){
							ch2 = getch();
							printf("%c%c", ch, ch2);
						}
						else{
						printf("%c", ch);
						}
						fflush(0);
						n++;
					}
					else if (pos_x == 118 & pos_y < 29){
						pos_y++;
						pos_x = 3;
						gotoxy(pos_x, pos_y);
						if (ch == 208 || ch == 209){
							ch2 = getch();
							printf("%c%c", ch, ch2);
						}
						else{
						printf("%c", ch);
						}
						fflush(0);
						n++;
					}
					resetcolor();
					
					//обработка начального символа команды
					if (n == 1){
						if (ch == '*' || ch == '@'){	
							command = ch;
						}
						else {
							print_bottom_window();
							pos_x = 2;
							pos_y = 27;
							gotoxy(pos_x, pos_y);
							fflush(0);
							n = 0;
							continue;
						}
					}
					else if (n == 2){
						if (command == '*'){
							if (ch != ' '){
								print_bottom_window();
								pos_x = 2;
								pos_y = 27;
								gotoxy(pos_x, pos_y);
								fflush(0);
								n = 0;
								continue;
							}
							else {
								flag_message = 1;
								continue;
							}
						}
						else if (command == '@'){
							if (ch != ' '){
								flag_name = 1;
								use_save_name = 0;
							}
							else {
								use_save_name = 1;
								flag_message = 1;
								continue;
							}
						}					
					}
					
					//обработка символов имени и сообщения
					if (flag_name == 1){
						if (ch != ' '){
							name[i] = (char)ch;
							i++;
							if (ch2 != 0){
							name[i] = (char)ch2;
							i++;
							ch2 = 0;
							}
						}
						else{
							flag_name = 0;
							i = 0;
							flag_message = 1;
						}
					}
					else if (flag_message == 1){
						message[i] = (char)ch;
						i++;
						if (ch2 != 0){
							message[i] = (char)ch2;
							i++;
							ch2 = 0;
						}
					}
				}
			}
		}
		//приём данных из сокета
		if (fds[1].revents & POLLIN) {
			gotoxy(3,2);
			memset(result, 0, sizeof(result));
			errwrap(recv(sock_fd, result, MAX_BUF_SIZE, 0));
			if (strlen(result) > 2){
				add_message(result);
			
				if (chat_pointer > 24 & read_end < chat_pointer){
					read_end = chat_pointer;
					read_start = read_end - 24;
				}
			
				print_messages();
				//printf("\a");
				time(&start);
				sleep(1);
				print_users(sock_fd);
				time(&start);
				gotoxy(pos_x,pos_y);
				fflush(0);
			}
		}
		time(&end);
		elapsed = difftime(end, start);
		//пинг сервера
		if (elapsed > 20){
			PING(sock_fd);
			time(&start);
		}				
	}
_exit:
	resetcolor();
	reset_input_mode();
    close(sock_fd);
	pos_x = 1;
	pos_y = 30;
	gotoxy(pos_x, pos_y);
	fflush(0);
}

int main(int argc, char **argv) {
	if (argc == 3){	
		char* login = argv[1];
		char* password = argv[2];
		main_loop(login, password);
	}
	else {
		printf("Неверные аргументы\nПример запуска: ./client your_name your_password\n");
	}
    return 0;
}
