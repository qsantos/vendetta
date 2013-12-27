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

	sfView* view;
};

#define SW_WIDTH  307.f
#define SW_HEIGHT 356.f

void subwindow_init(subwindow_t* w, graphics_t* g, const wchar_t* name, float x, float y);
void subwindow_exit(subwindow_t* w);

char subwindow_cursor(subwindow_t* w, int x, int y);
char subwindow_draw  (subwindow_t* w, graphics_t* g);
char subwindow_catch (subwindow_t* w, int x, int y, int t);
char subwindow_wheel (subwindow_t* w, int x, int y, int delta);

#endif
