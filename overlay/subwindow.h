#ifndef O_SUBWINDOW_H
#define O_SUBWINDOW_H

typedef struct subwindow subwindow_t;

#include "../graphics.h"

struct subwindow
{
	char visible;
	float x;
	float y;
	const wchar_t* name;
};

#define SW_WIDTH  307
#define SW_HEIGHT 356

void subwindow_init(subwindow_t* w, const wchar_t* name, float x, float y);
void subwindow_exit(subwindow_t* w);

char subwindow_draw(subwindow_t* w, graphics_t* g);

#endif
