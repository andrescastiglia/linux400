#include <stdio.h>

extern void init();

int main() {
    printf("Hola Mundo desde Linux/400 (C/400)\n");
    init(); // Llamada a la runtime L400
    return 0;
}
