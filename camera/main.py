# Blue bot
import sensor, time, math, struct, pyb
from pyb import UART

sensor.reset()
sensor.set_pixformat(sensor.RGB565)
sensor.set_framesize(sensor.QVGA)

centre_x = 162
centre_y = 110

sensor.set_windowing((centre_x-120, 0, 240, 240))
sensor.set_gainceiling(128)
sensor.set_auto_gain(False)
sensor.set_auto_whitebal(False) # must be turned off for color tracking

sensor.skip_frames(time=500)
sensor.set_auto_exposure(False, exposure_us=8000)
sensor.set_auto_gain(False, gain_db = 10)
sensor.set_auto_whitebal(False)
sensor.set_framebuffers(3)

clock = time.clock()  # Create a clock object to track the FPS.
clock.reset()
led1 = pyb.LED(1)
led2 = pyb.LED(2)
led3 = pyb.LED(3)
#led1.on()
led2.on()
#led3.on()

centre_x = 125
centre_y = 126

# from sci centre
#thresh_ball = (0, 100, 20, 58, 39, 61)
#thresh_ball = (8, 97, 5, 54, 36, 42)
#thresh_ball = (13, 62, 33, 62, 17, 75) # ground floor
thresh_ball = (14, 62, 54, 75, 11, 41)
# old thresh 6/4 (54, 99, 7, 72, 11, 73) # (40, 68, 14, 57, 9, 59)
# old ball threshes (49, 69, 7, 55, 0, 29) # (45, 69, 3, 60, 19, 65)
thresh_yellow_goal = (39, 100, -24, 3, 35, 67) # (63, 81, -18, 8, 30, 67)
thresh_blue_goal = (0, 100, -10, 0, -35, -10) # (57, 75, -30, 3, -35, -19) # (49, 58, -29, 3, -34, -10)

#thresh_ball = (35, 77, 7, 50, 25, 65) # "The good one"
# thresh_ball = (43, 76, 3, 44, 4, 67) # (43, 76, 2, 24, 4, 35)
# thresh_ball = (46, 75, 15, 38, -2, 65) # (52, 75, 14, 48, 21, 50)
# thresh_yellow_goal = (45, 100, -25, 15, 24, 72) # (64, 92, -29, -3, 54, 85)
# thresh_blue_goal = (46, 58, -27, -7, -34, -17)

# fitting dist
# calib dists = [0, 26, 27.0185, 38, 47, 55, 62.5, 69, 74, 77.5, 81.5, 85.5, 88.0909, 90.5, 93.1343, 94.5, 96.1301, 97.1288, 98.1275, 98.5, 99.1262, 99.5, 100.125, 101, 102.122, 102.176]

# dists = [0, 33, 45, 54.015, 63.5, 71.5, 79, 83.5, 87.5057, 92.0054, 95.5, 98.5, 101.5, 103.621, 104.62, 106.075, 106.118, 107.168, 108.227, 109.195, 109.573, 110.114, 111.113, 111.613, 112.112, 112.612, 113.111]
# Y = 0.0001144220513222588 * e^(0.11943230322950865 * x) + 0.48308099078412714 * x + -4.1580139472465865
m, t, c, d = 0.0001144220513222588, 0.11943230322950865, 0.48308099078412714, -4.1580139472465865

uart = UART(3, 115200)
uart.init(115200, bits=8, parity=None, stop=1, timeout_char=1000)

# find use goal
img = sensor.snapshot()
yellow_goal = img.find_blobs([thresh_yellow_goal], pixel_threshold=100, area_threshold=0, merge=True)
blue_goal = img.find_blobs([thresh_blue_goal], pixel_threshold=100, area_threshold=0, merge=True, margin=20)
while(len(yellow_goal)<1 or len(blue_goal)<1): # here
    img = sensor.snapshot()
    yellow_goal = img.find_blobs([thresh_yellow_goal], pixel_threshold=100, area_threshold=0, merge=True)
    blue_goal = img.find_blobs([thresh_blue_goal], pixel_threshold=100, area_threshold=0, merge=True, margin=20)
    break;
#bg = max(blue_goal, key = lambda bg:bg.pixels())
#yg = max(yellow_goal, key = lambda yg:yg.pixels())
#if(yg.cy()<centre_y):
#    use_goal = "blue"
#else:
#    use_goal = "yellow"
#led2.off()
#use_blue = yg.cy() < centre_y  # here
use_blue = True
while True:
    clock.tick()  # Update the FPS clock.
    led1.off()
    led3.off()
    img = sensor.snapshot()  # Take a picture and return the image.
    # img.draw_rectangle(83, 79, 73, 73, color=(0,0,0), fill=True)
    # img.mask_circle(120, 120, 112)
    ball = img.find_blobs([thresh_ball], pixel_threshold=0, area_threshold=0, merge=True)
#    if(not use_blue):
#        yellow_goal = img.find_blobs([thresh_yellow_goal], pixel_threshold=100, area_threshold=0, merge=True, margin=20)
#    else:
#        blue_goal = img.find_blobs([thresh_blue_goal], pixel_threshold=100, area_threshold=0, merge=True, margin=20)
    img.draw_cross(centre_x, centre_y)
    # print(sensor.get_exposure_us())
    use_goal = "nil"

    no_ball = False
    no_goal = False

    if len(ball)>0:
        led2.off()
        led1.on()
        # print("ball")
        b = max(ball, key = lambda b:b.pixels())
        img.draw_rectangle(b.rect())
        ball_x = b.cx() - centre_x
        ball_y = b.cy() - centre_y
        ball_angle = math.atan2(ball_x, ball_y) * 180 / math.pi - 90
        if ball_angle<0:
            ball_angle += 360
        ball_dist = (ball_x ** 2 + ball_y ** 2) ** 0.5
        print("angle ", ball_angle, "dist ", ball_dist)
        actual_dist = m * math.exp(t*ball_dist) + c*ball_dist + d
#        print("actual", actual_dist)
        angle_uart = round(ball_angle * 128)
        dist_uart = round(actual_dist * 128)
        # time.sleep(0.5)
    else:
        no_ball = True

    if (use_blue and len(blue_goal)>0):
        led2.off()
        led3.on()
        # print("blue goal")
        bg = max(blue_goal, key = lambda bg:bg.pixels())
        img.draw_rectangle(bg.rect())
        img.draw_cross(bg.cx(), bg.cy())

        bgoal_x = bg.cx() - centre_x
        bgoal_y = bg.cy() - centre_y
        bgoal_angle = math.atan2(bgoal_x, bgoal_y) * 180 / math.pi
        if bgoal_angle<0:
            bgoal_angle += 360

        bgoal_dist = m * math.exp(t*abs(bgoal_y)) + c*abs(bgoal_y) + d
        # print("blue dist", bgoal_dist)

        angle_bgoal_uart = round(bgoal_angle * 128)
        dist_bgoal_uart = round(bgoal_dist * 128)
    else:
        no_goal = True

    if ((not use_blue) and len(yellow_goal)>0):
        led2.off()
        led3.on()
        # print("yellow goal")
        yg = max(yellow_goal, key = lambda yg:yg.pixels())
        img.draw_rectangle(yg.rect())
        img.draw_cross(yg.cx(), yg.cy())

        ygoal_x = yg.cx() - centre_x
        ygoal_y = yg.cy() - centre_y
        ygoal_angle = math.atan2(ygoal_x, ygoal_y) * 180 / math.pi
        if ygoal_angle<0:
            ygoal_angle += 360

        ygoal_dist = m * math.exp(t*abs(ygoal_y)) + c*abs(ygoal_y) + d
        # print("yellow dist", ygoal_dist)

        angle_ygoal_uart = round(ygoal_angle * 128)
        dist_ygoal_uart = round(ygoal_dist * 128)
    else:
        no_goal = True

#    if len(goal)>0:
#        # print("goal")
#        bg = max(goal, key = lambda bg:bg.pixels())
#        img.draw_rectangle(bg.rect())
#        img.draw_cross(bg.cx(), bg.cy())
#        if len(goal)>1:
#            goal.sort(key=lambda goal:goal.pixels(), reverse=True)
#            if goal[1].pixels()>150:
#                img.draw_rectangle(goal[0].rect())
#                img.draw_rectangle(goal[1].rect())
#                total_blue_area = goal[0].area() + goal[1].area()
#                small_blue_x = min(goal[0].cx(), goal[1].cx())
#                small_blue_y = min(goal[0].cy(), goal[1].cy())
#                blue_x = small_blue_x + abs(goal[0].cx()-goal[1].cx()) * ((goal[1].area() / total_blue_area)**0.5)
#                blue_y = small_blue_y + abs(goal[0].cy()-goal[1].cy()) * ((goal[1].area() / total_blue_area)**0.5)
#                img.draw_cross(math.floor(blue_x), math.floor(blue_y))
#            else:
#                img.draw_cross(bg.cx(), bg.cy())
#        elif len(goal)==1:
#            img.draw_cross(bg.cx(), bg.cy())
    uart.writechar(1)
    if(no_ball==False):
        uart.writechar(angle_uart & 0xFF)
        uart.writechar((angle_uart >> 8) & 0xFF)
        uart.writechar(dist_uart & 0xFF)
        uart.writechar((dist_uart >> 8) & 0xFF)
    else:
        uart.writechar(0)
        uart.writechar(0)
        uart.writechar(0)
        uart.writechar(0)
    if(no_goal==False):
        if(use_goal=="blue"):
            uart.writechar(angle_bgoal_uart & 0xFF)
            uart.writechar((angle_bgoal_uart >> 8) & 0xFF)
            uart.writechar(dist_bgoal_uart & 0xFF)
            uart.writechar((dist_bgoal_uart >> 8) & 0xFF)
        else:
            uart.writechar(angle_ygoal_uart & 0xFF)
            uart.writechar((angle_ygoal_uart >> 8) & 0xFF)
            uart.writechar(dist_ygoal_uart & 0xFF)
            uart.writechar((dist_ygoal_uart >> 8) & 0xFF)
    else:
        uart.writechar(0)
        uart.writechar(0)
        uart.writechar(0)
        uart.writechar(0)
    uart.sendbreak()

#    print("fps", clock.fps())
