#ifndef O_SUBWINDOW_H
#define O_SUBWINDOW_H

typedef struct subwindow subwindow_t;

#include "../graphics.h"

struct subwindow
{
	const wchar_t* name;
	float x;
	float y;
};

#define SW_WIDTH  307
#define SW_HEIGHT 356

void subwindow_init(subwindow_t* w, const wchar_t* name, float x, float y);
void subwindow_exit(subwindow_t* w);

void subwindow_draw(subwindow_t* w, graphics_t* g);

#endif
