#ifndef O_SUBWINDOW_H
#define O_SUBWINDOW_H

typedef struct subwindow subwindow_t;

#include "../graphics.h"

struct subwindow
{
	float x;
	float y;
};

void subwindow_init(subwindow_t* w);
void subwindow_exit(subwindow_t* w);

void subwindow_draw(subwindow_t* w, graphics_t* g);

#endif
